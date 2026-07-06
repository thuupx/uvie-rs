//! Coda (final consonant cluster) validation and tone-coda constraints.

/// Legal Vietnamese final consonant clusters (raw ASCII).
///
/// Standard Vietnamese final consonants:
/// `{T}, {P}, {C}, {N}, {M}, {NG}, {NH}, {CH}`
///
/// Note: `ng`, `nh`, `ch` are stored as 2-byte slices; single finals as 1-byte.
/// The key `c` can represent final /k/ (before ă/â) *or* final /c/ - both legal.
#[allow(dead_code)]
pub(crate) static LEGAL_CODAS: &[&[u8]] = &[
    b"t", b"p", b"c", b"n", b"m", // digraph codas
    b"ng", b"nh", b"ch", // glide finals - 'i' and 'y' act as coda in oai, oay, etc.
    b"i", b"y", b"u",
    // 'o' appears as coda in "ao", "eo" etc. (vowel clusters handle this via
    // the nucleus table, but some patterns treat 'o' as a trailing glide)
    b"o",
];

/// Returns `true` if `coda` (slice of raw base bytes after the nucleus) is a
/// legal Vietnamese final cluster.  The empty coda is always legal.
///
/// When `relaxed` is `true`, the following lone finals are also accepted as
/// legal codas (rendered verbatim — the user types them as a shorthand for the
/// digraph but the engine keeps the typed char):
///   - `g` (shorthand for `ng`, e.g. "đặg")
///   - `h` (shorthand for `nh`, e.g. "nhàh")
///
/// The following teen-code shorthands are **always active** (no toggle needed),
/// rendered verbatim like the relaxed shorthands above:
///   - `k`  (shorthand for `c`,  e.g. "đắk" → Đắk Lắk province spelling)
///   - `nk` (shorthand for `nh`, e.g. "đỉnk" teen code for "đỉnh")
pub fn is_legal_coda(coda: &[u8], relaxed: bool) -> bool {
    match coda.len() {
        0 => true,
        1 => match coda[0] {
            b't' | b'p' | b'c' | b'n' | b'm' | b'i' | b'y' | b'u' | b'o' => true,
            // Teen-code shorthand for `c` — always active (e.g. "đắk").
            b'k' => true,
            b'g' | b'h' if relaxed => true,
            _ => false,
        },
        2 => match coda {
            b"ng" | b"nh" | b"ch" => true,
            // Teen-code shorthand for `nh` — always active (e.g. "đỉnk").
            b"nk" => true,
            _ => false,
        },
        _ => false,
    }
}

/// Tone-coda phonotactic constraint.
///
/// In Vietnamese orthography:
/// - Codas `c`, `ch`, `p`, `t` only allow tones sắc (1) and nặng (5).
/// - All other codas (or empty coda) allow any tone.
///
/// Vietnamese phonotactic rule: stopped codas only allow sắc/nặng tones.
///
/// In `relaxed` mode, a lone `g` coda is treated as shorthand for `ng` and a
/// lone `h` coda as shorthand for `nh`; both therefore allow any tone.
///
/// The teen-code shorthands `k` (for `c`) and `nk` (for `nh`) are always
/// active: `k` follows the stopped-coda rule (sắc/nặng only) while `nk`
/// behaves like `nh` (any tone).
pub fn tone_allowed_for_coda(coda: &[u8], tone: u8, relaxed: bool) -> bool {
    if tone == 0 {
        return true; // bằng / no-tone always OK
    }
    match coda.len() {
        0 => true,
        1 => {
            if relaxed && matches!(coda[0], b'g' | b'h') {
                return true;
            }
            // `k` is the stopped coda /k/ (shorthand for `c`): sắc/nặng only.
            if matches!(coda[0], b'c' | b'p' | b't' | b'k') {
                matches!(tone, 1 | 5)
            } else {
                true
            }
        }
        2 => {
            if coda == b"ch" {
                matches!(tone, 1 | 5)
            } else {
                // `nk` is shorthand for `nh` — any tone allowed.
                true
            }
        }
        _ => true,
    }
}
