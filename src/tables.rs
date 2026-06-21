//! Positive Vietnamese syllable pattern tables.
//!
//! Positive onset / nucleus / coda tables for Vietnamese syllable validation.
//!
//! # Validation strategy
//!
//! A word is Vietnamese iff:
//!   `is_legal_onset(onset) && nucleus_tone_target(nucleus).is_some() && is_legal_coda(coda)`
//!
//! Anything that does not match falls through as **literal passthrough**. This
//! automatically handles English words without any blacklist.
//!
//! # Tone-target index
//!
//! `nucleus_tone_target` returns `Some(i)` where `i` is the 0-based offset
//! within the nucleus slice that should receive the tone diacritic (modern
//! orthography). This replaces the 60-line `apply_tone_in_place` heuristic.
//!
//! # Sources
//!
//! - Vietnamese orthography standard (onset/coda/nucleus constraints).
//! - Cross-referenced against `src/tests.rs` for tone-placement tests.

// ---------------------------------------------------------------------------
// §1 Onset (initial consonant cluster)
// ---------------------------------------------------------------------------

/// Legal Vietnamese initial consonant clusters (raw ASCII).
///
/// The empty onset (word starts with a vowel) is handled by the caller.
/// Single-char consonants b/c/d/g/h/k/l/m/n/p/q/r/s/t/v/x are all legal;
/// they are listed here as 1-char entries. Multi-char clusters are explicit.
///
/// Standard Vietnamese consonant clusters:
/// ```text
/// {NGH}, {PH}, {TH}, {TR}, {GI}, {CH}, {NH}, {NG}, {KH}, {GH},
/// {G}, {C}, {Q}, {K}, {T}, {R}, {H}, {B}, {M}, {V}, {N}, {L},
/// {X}, {P}, {S}, {D}, (F/W/Z/J as foreign/special)
/// ```
static LEGAL_ONSETS: &[&[u8]] = &[
    // 3-char
    b"ngh", // 2-char
    b"ph", b"th", b"tr", b"gi", b"ch", b"nh", b"ng", b"kh", b"gh", b"qu",
    // 1-char (all legal single-consonant onsets)
    b"b", b"c", b"d", b"g", b"h", b"k", b"l", b"m", b"n", b"p", b"q", b"r", b"s", b"t", b"v", b"x",
    // đ (base 'd', but in practice the engine stores raw 'd' for đ onset too)
    b"d",
    // Foreign/extended allowed as onset
    // NOTE: 'f' removed to fix "fix" -> "fĩ" bug (English word interference)
    b"w", b"z", b"j",
];

/// Returns `true` if `onset` (slice of raw base bytes before the nucleus) is
/// a legal Vietnamese initial cluster.  The empty onset is always legal.
pub fn is_legal_onset(onset: &[u8]) -> bool {
    match onset.len() {
        0 => true,
        1 => {
            let b = onset[0];
            // Any single lowercase consonant that is not a pure vowel key
            // NOTE: 'f' excluded to fix "fix" -> "fĩ" bug (English word interference)
            matches!(
                b,
                b'b' | b'c'
                    | b'd'
                    | b'g'
                    | b'h'
                    | b'j'
                    | b'k'
                    | b'l'
                    | b'm'
                    | b'n'
                    | b'p'
                    | b'q'
                    | b'r'
                    | b's'
                    | b't'
                    | b'v'
                    | b'w'
                    | b'x'
                    | b'z'
            )
        }
        2 => {
            // Explicit 2-char whitelist
            matches!(
                onset,
                b"ph" | b"th" | b"tr" | b"gi" | b"ch" | b"nh" | b"ng" | b"kh" | b"gh" | b"qu"
            )
        }
        3 => onset == b"ngh",
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// §2 Coda (final consonant cluster)
// ---------------------------------------------------------------------------

/// Legal Vietnamese final consonant clusters (raw ASCII).
///
/// Standard Vietnamese final consonants:
/// `{T}, {P}, {C}, {N}, {M}, {NG}, {NH}, {CH}`
///
/// Note: `ng`, `nh`, `ch` are stored as 2-byte slices; single finals as 1-byte.
/// The key `c` can represent final /k/ (before ă/â) *or* final /c/ - both legal.
#[allow(dead_code)]
static LEGAL_CODAS: &[&[u8]] = &[
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
/// When `relaxed` is `true`, the final `g` is also accepted as shorthand for
/// `ng` (e.g. "đặg" for "đặng").
pub fn is_legal_coda(coda: &[u8], relaxed: bool) -> bool {
    match coda.len() {
        0 => true,
        1 => match coda[0] {
            b't' | b'p' | b'c' | b'n' | b'm' | b'i' | b'y' | b'u' | b'o' => true,
            b'g' if relaxed => true,
            _ => false,
        },
        2 => matches!(coda, b"ng" | b"nh" | b"ch"),
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
/// In `relaxed` mode, a lone `g` coda is treated as shorthand for `ng` and
/// therefore allows any tone.
pub fn tone_allowed_for_coda(coda: &[u8], tone: u8, relaxed: bool) -> bool {
    if tone == 0 {
        return true; // bằng / no-tone always OK
    }
    match coda.len() {
        0 => true,
        1 => {
            if coda[0] == b'g' && relaxed {
                return true;
            }
            if matches!(coda[0], b'c' | b'p' | b't') {
                matches!(tone, 1 | 5)
            } else {
                true
            }
        }
        2 => {
            if coda == b"ch" {
                matches!(tone, 1 | 5)
            } else {
                true
            }
        }
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// §3 Nucleus (vowel core) with tone-target index
// ---------------------------------------------------------------------------

/// A nucleus entry: the resolved vowel sequence and the index within that
/// sequence that should receive the tone diacritic (modern orthography).
///
/// `seq` uses resolved characters (after circumflex/horn transform):
/// `'â'`, `'ê'`, `'ô'`, `'ă'`, `'ơ'`, `'ư'`, plain `'a'`/`'e'`/`'i'`/`'o'`/`'u'`/`'y'`.
///
/// `tone_idx` is 0-based offset within `seq`.
struct NucleusEntry {
    /// Resolved vowel characters in order (up to 3).
    seq: &'static [char],
    /// Index within `seq` that receives the tone diacritic.
    tone_idx: usize,
}

/// All legal Vietnamese nuclei with their tone-target positions.
///
/// Sorted longest-first so the search loop can do prefix-match correctly.
///
/// Sources:
/// - Vietnamese orthography standard (modern style: tone on "main" vowel)
///
/// Tone-target rules (modern orthography):
/// - Triphthongs `iêu`, `oai`, `oao`, `uya`, `uyê`, `ươi`, `ươu`:
///   tone on index 1 (middle vowel).
/// - Diphthongs `ia`, `iê`, `ua`, `uâ`, `uê`, `uô`, `ưa`, `ươ`, `uy`, `yê`:
///   tone on index 0 (first vowel = the "heavier" one).
/// - Diphthongs `oa`, `oe`, `oi`, `oo`, `ôi`, `ơi`, `ai`, `ao`, `au`, `ay`,
///   `âu`, `ây`, `êu`, `ôi`, `oi`, `ui`, `ưi`, `ưu`:
///   tone on the vowel that is NOT the final glide, typically index 0.
/// - Single vowels: index 0.
///
/// The `qu`/`gi` special case:
/// after `qu`, the `u` is a glide (not nucleus), so `qua` nucleus = `[a]`,
/// tone-target = 0. After `gi`, the `i` is a glide, so `gia` nucleus = `[a]`.
/// This is handled at the engine level, not here.
static NUCLEUS_TABLE: &[NucleusEntry] = &[
    // ---- Triphthongs (3 vowels) ----
    NucleusEntry {
        seq: &['i', 'ê', 'u'],
        tone_idx: 1,
    }, // iêu
    NucleusEntry {
        seq: &['o', 'a', 'i'],
        tone_idx: 1,
    }, // oai
    NucleusEntry {
        seq: &['o', 'a', 'o'],
        tone_idx: 1,
    }, // oao (rare, e.g. "loạo")
    NucleusEntry {
        seq: &['o', 'a', 'y'],
        tone_idx: 1,
    }, // oay
    NucleusEntry {
        seq: &['u', 'y', 'a'],
        tone_idx: 1,
    }, // uya
    NucleusEntry {
        seq: &['u', 'y', 'ê'],
        tone_idx: 2,
    }, // uyê (e.g. quyết → nucleus=uyê, tone→ê)
    NucleusEntry {
        seq: &['u', 'y', 'u'],
        tone_idx: 1,
    }, // uyu
    NucleusEntry {
        seq: &['ư', 'ơ', 'i'],
        tone_idx: 1,
    }, // ươi
    NucleusEntry {
        seq: &['ư', 'ơ', 'u'],
        tone_idx: 1,
    }, // ươu
    NucleusEntry {
        seq: &['u', 'ô', 'i'],
        tone_idx: 1,
    }, // uôi (cuối, muối, etc.)
    NucleusEntry {
        seq: &['y', 'ê', 'u'],
        tone_idx: 1,
    }, // yêu
    // ---- Diphthongs (2 vowels) - tone on first (the "main") vowel ----
    // Modified-vowel diphthongs first (more specific)
    NucleusEntry {
        seq: &['â', 'u'],
        tone_idx: 0,
    }, // âu
    NucleusEntry {
        seq: &['â', 'y'],
        tone_idx: 0,
    }, // ây
    NucleusEntry {
        seq: &['â', 'o'],
        tone_idx: 0,
    }, // âo (nấo etc.)
    NucleusEntry {
        seq: &['ă', 'y'],
        tone_idx: 0,
    }, // ăy (rare)
    NucleusEntry {
        seq: &['ă', 'u'],
        tone_idx: 0,
    }, // ău (tầu - boat)
    NucleusEntry {
        seq: &['o', 'ă'],
        tone_idx: 1,
    }, // oă (hoăng, loăng quăng)
    NucleusEntry {
        seq: &['ê', 'o'],
        tone_idx: 0,
    }, // êo (rare)
    NucleusEntry {
        seq: &['ê', 'u'],
        tone_idx: 0,
    }, // êu (nếu → tone on ê)
    NucleusEntry {
        seq: &['ô', 'i'],
        tone_idx: 0,
    }, // ôi
    NucleusEntry {
        seq: &['ô', 'u'],
        tone_idx: 0,
    }, // ôu (rare)
    NucleusEntry {
        seq: &['ơ', 'i'],
        tone_idx: 0,
    }, // ơi
    NucleusEntry {
        seq: &['ơ', 'u'],
        tone_idx: 0,
    }, // ơu (rare)
    NucleusEntry {
        seq: &['ư', 'a'],
        tone_idx: 0,
    }, // ưa
    NucleusEntry {
        seq: &['ư', 'i'],
        tone_idx: 0,
    }, // ưi (gửi → tone on ư)
    NucleusEntry {
        seq: &['ư', 'o'],
        tone_idx: 0,
    }, // ưo (rare)
    NucleusEntry {
        seq: &['ư', 'u'],
        tone_idx: 0,
    }, // ưu
    NucleusEntry {
        seq: &['ư', 'ơ'],
        tone_idx: 1,
    }, // ươ (hướng → tone on ơ, index 1)
    NucleusEntry {
        seq: &['u', 'ô'],
        tone_idx: 1,
    }, // uô: tone on ô (nuốt, thuốc, etc.)
    // plain-vowel diphthongs
    NucleusEntry {
        seq: &['i', 'a'],
        tone_idx: 0,
    }, // ia (mía → tone on i)
    NucleusEntry {
        seq: &['i', 'ê'],
        tone_idx: 1,
    }, // iê / yê (tiến → tone on ê)
    NucleusEntry {
        seq: &['i', 'o'],
        tone_idx: 0,
    }, // io (rare)
    NucleusEntry {
        seq: &['y', 'ê'],
        tone_idx: 1,
    }, // yê (huyền → tone on ê)
    NucleusEntry {
        seq: &['u', 'a'],
        tone_idx: 0,
    }, // ua (múa → tone on u)
    NucleusEntry {
        seq: &['u', 'â'],
        tone_idx: 1,
    }, // uâ - tone on â (chuẩn, tuần)
    NucleusEntry {
        seq: &['u', 'ê'],
        tone_idx: 1,
    }, // uê (quê → tone on ê)
    NucleusEntry {
        seq: &['u', 'y'],
        tone_idx: 1,
    }, // uy (tuỳ → tone on y in modern ortho)
    NucleusEntry {
        seq: &['u', 'i'],
        tone_idx: 0,
    }, // ui
    NucleusEntry {
        seq: &['u', 'o'],
        tone_idx: 0,
    }, // uo (vuốt → tone on u, but uo is often ươ)
    NucleusEntry {
        seq: &['u', 'u'],
        tone_idx: 0,
    }, // uu (transient state for uuw → ưu)
    NucleusEntry {
        seq: &['a', 'i'],
        tone_idx: 0,
    }, // ai
    NucleusEntry {
        seq: &['a', 'o'],
        tone_idx: 0,
    }, // ao
    NucleusEntry {
        seq: &['a', 'u'],
        tone_idx: 0,
    }, // au
    NucleusEntry {
        seq: &['a', 'y'],
        tone_idx: 0,
    }, // ay
    NucleusEntry {
        seq: &['e', 'o'],
        tone_idx: 0,
    }, // eo
    NucleusEntry {
        seq: &['i', 'u'],
        tone_idx: 0,
    }, // iu
    NucleusEntry {
        seq: &['o', 'a'],
        tone_idx: 1,
    }, // oa (hoá → tone on a)
    NucleusEntry {
        seq: &['o', 'e'],
        tone_idx: 1,
    }, // oe (hoè → tone on e)
    NucleusEntry {
        seq: &['o', 'i'],
        tone_idx: 0,
    }, // oi (tối → tone on o or ô)
    NucleusEntry {
        seq: &['o', 'o'],
        tone_idx: 0,
    }, // oo (kept for double-o sequences)
    // ---- Single vowels (always tone-idx 0) ----
    NucleusEntry {
        seq: &['a'],
        tone_idx: 0,
    },
    NucleusEntry {
        seq: &['ă'],
        tone_idx: 0,
    },
    NucleusEntry {
        seq: &['â'],
        tone_idx: 0,
    },
    NucleusEntry {
        seq: &['e'],
        tone_idx: 0,
    },
    NucleusEntry {
        seq: &['ê'],
        tone_idx: 0,
    },
    NucleusEntry {
        seq: &['i'],
        tone_idx: 0,
    },
    NucleusEntry {
        seq: &['o'],
        tone_idx: 0,
    },
    NucleusEntry {
        seq: &['ô'],
        tone_idx: 0,
    },
    NucleusEntry {
        seq: &['ơ'],
        tone_idx: 0,
    },
    NucleusEntry {
        seq: &['u'],
        tone_idx: 0,
    },
    NucleusEntry {
        seq: &['ư'],
        tone_idx: 0,
    },
    NucleusEntry {
        seq: &['y'],
        tone_idx: 0,
    },
];

/// Returns `Some(tone_target_index)` if `nucleus` is a legal Vietnamese vowel
/// core, where the index is the position within `nucleus` that receives the
/// tone mark (modern orthography).
///
/// Returns `None` if the vowel sequence is not a legal Vietnamese nucleus.
///
/// `nucleus` must contain resolved characters (after circumflex/horn transform),
/// not raw input keys.
pub fn nucleus_tone_target(nucleus: &[char]) -> Option<usize> {
    if nucleus.is_empty() {
        return None;
    }
    // Linear scan; NUCLEUS_TABLE is ~50 entries and nucleus ≤ 3 chars.
    for entry in NUCLEUS_TABLE {
        if entry.seq == nucleus {
            return Some(entry.tone_idx);
        }
    }
    None
}

/// Returns `true` if `nucleus` is any legal Vietnamese vowel core (ignores
/// tone-target position). Equivalent to `nucleus_tone_target(n).is_some()`.
#[inline]
pub fn is_legal_nucleus(nucleus: &[char]) -> bool {
    nucleus_tone_target(nucleus).is_some()
}

// ---------------------------------------------------------------------------
// §4 Full syllable validation
// ---------------------------------------------------------------------------

/// Validates a complete syllable given its three components expressed as
/// resolved output characters.
///
/// - `onset_raw`:   raw base bytes of the onset (e.g. `b"th"`, `b"ngh"`).
/// - `nucleus_out`: resolved chars of the nucleus (e.g. `['ê']`, `['o','a']`).
/// - `coda_raw`:    raw base bytes of the coda (e.g. `b"ng"`, `b"t"`).
///
/// Returns `true` if all three components are legal and the tone is compatible
/// with the coda.
pub fn is_legal_syllable(
    onset_raw: &[u8],
    nucleus_out: &[char],
    coda_raw: &[u8],
    tone: u8,
    relaxed: bool,
) -> bool {
    is_legal_onset(onset_raw)
        && is_legal_nucleus(nucleus_out)
        && is_legal_coda(coda_raw, relaxed)
        && tone_allowed_for_coda(coda_raw, tone, relaxed)
}

// ---------------------------------------------------------------------------
// §5 Prefix-validity (used for incremental validation)
// ---------------------------------------------------------------------------

/// Returns `true` if the given raw onset bytes form a **legal prefix** of some
/// Vietnamese onset - i.e., the onset is valid as-is OR could become valid with
/// more keystrokes.
///
/// Used to decide whether to keep composing or fall through to English
/// passthrough on the very first consonants.
pub fn is_onset_prefix(prefix: &[u8]) -> bool {
    if prefix.is_empty() {
        return true;
    }
    // Check exact match first.
    if is_legal_onset(prefix) {
        return true;
    }
    // Check if any legal onset starts with `prefix`.
    for onset in LEGAL_ONSETS {
        if onset.starts_with(prefix) {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// §6 qu / gi glide detection
// ---------------------------------------------------------------------------

/// Returns `true` if the onset is `qu` (so the following `u` is a glide, not
/// a nucleus vowel).
#[inline]
pub fn onset_is_qu(onset_raw: &[u8]) -> bool {
    onset_raw == b"qu"
}

/// Returns `true` if the onset is `gi` (so the following `i` is a glide, not
/// a nucleus vowel). `d` is the raw key for `đ` as onset - `gi` raw is `gi`.
#[inline]
pub fn onset_is_gi(onset_raw: &[u8]) -> bool {
    onset_raw == b"gi"
}

// ---------------------------------------------------------------------------
// §7 Character-class helpers (complement modes.rs; no IME tables needed here)
// ---------------------------------------------------------------------------

/// Returns `true` if `c` is a Vietnamese vowel base character (resolved).
#[inline]
pub fn is_vowel_base(c: char) -> bool {
    matches!(
        c,
        'a' | 'ă' | 'â' | 'e' | 'ê' | 'i' | 'o' | 'ô' | 'ơ' | 'u' | 'ư' | 'y'
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onset_single_chars() {
        assert!(is_legal_onset(b"b"));
        assert!(is_legal_onset(b"t"));
        assert!(is_legal_onset(b"n"));
        assert!(is_legal_onset(b""));
    }

    #[test]
    fn onset_digraphs() {
        assert!(is_legal_onset(b"ph"));
        assert!(is_legal_onset(b"th"));
        assert!(is_legal_onset(b"tr"));
        assert!(is_legal_onset(b"gi"));
        assert!(is_legal_onset(b"ch"));
        assert!(is_legal_onset(b"nh"));
        assert!(is_legal_onset(b"ng"));
        assert!(is_legal_onset(b"kh"));
        assert!(is_legal_onset(b"gh"));
        assert!(is_legal_onset(b"qu"));
    }

    #[test]
    fn onset_ngh() {
        assert!(is_legal_onset(b"ngh"));
    }

    #[test]
    fn onset_illegal() {
        assert!(!is_legal_onset(b"tt")); // double consonant
        assert!(!is_legal_onset(b"cl")); // English cluster
        assert!(!is_legal_onset(b"str")); // 3-char non-ngh
        assert!(!is_legal_onset(b"bl"));
    }

    #[test]
    fn coda_single() {
        assert!(is_legal_coda(b"t", false));
        assert!(is_legal_coda(b"n", false));
        assert!(is_legal_coda(b"m", false));
        assert!(is_legal_coda(b"c", false));
        assert!(is_legal_coda(b"p", false));
        assert!(is_legal_coda(b"i", false));
        assert!(is_legal_coda(b"y", false));
        assert!(is_legal_coda(b"u", false));
        assert!(is_legal_coda(b"", false));
        // relaxed mode: lone g as shorthand for ng
        assert!(is_legal_coda(b"g", true));
        assert!(!is_legal_coda(b"g", false));
    }

    #[test]
    fn coda_digraph() {
        assert!(is_legal_coda(b"ng", false));
        assert!(is_legal_coda(b"nh", false));
        assert!(is_legal_coda(b"ch", false));
    }

    #[test]
    fn coda_illegal() {
        assert!(!is_legal_coda(b"tt", false));
        assert!(!is_legal_coda(b"ll", false));
        assert!(!is_legal_coda(b"ngg", false));
    }

    #[test]
    fn tone_coda_constraint() {
        // c/ch/p/t only allow sắc(1) and nặng(5)
        assert!(tone_allowed_for_coda(b"c", 1, false));
        assert!(tone_allowed_for_coda(b"c", 5, false));
        assert!(!tone_allowed_for_coda(b"c", 3, false));
        assert!(!tone_allowed_for_coda(b"c", 4, false));
        assert!(tone_allowed_for_coda(b"ch", 1, false));
        assert!(!tone_allowed_for_coda(b"ch", 3, false));
        // n/m/ng are free
        assert!(tone_allowed_for_coda(b"n", 3, false));
        assert!(tone_allowed_for_coda(b"ng", 4, false));
        assert!(tone_allowed_for_coda(b"", 3, false));
        // relaxed mode: lone g behaves like ng
        assert!(tone_allowed_for_coda(b"g", 3, true));
        assert!(tone_allowed_for_coda(b"g", 4, true));
    }

    #[test]
    fn nucleus_single_vowels() {
        for &v in &['a', 'ă', 'â', 'e', 'ê', 'i', 'o', 'ô', 'ơ', 'u', 'ư', 'y'] {
            assert_eq!(
                nucleus_tone_target(&[v]),
                Some(0),
                "vowel {:?} should be legal",
                v
            );
        }
    }

    #[test]
    fn nucleus_diphthongs_tone_target() {
        // oa → tone on a (index 1): "hoá" = hoas
        assert_eq!(nucleus_tone_target(&['o', 'a']), Some(1));
        // oe → tone on e (index 1): "hoè"
        assert_eq!(nucleus_tone_target(&['o', 'e']), Some(1));
        // ưi → tone on ư (index 0): "gửi"
        assert_eq!(nucleus_tone_target(&['ư', 'i']), Some(0));
        // êu → tone on ê (index 0): "nếu"
        assert_eq!(nucleus_tone_target(&['ê', 'u']), Some(0));
        // iê → tone on ê (index 1): "tiến"
        assert_eq!(nucleus_tone_target(&['i', 'ê']), Some(1));
        // uy → tone on y (index 1): "tuỳ" (modern orthography)
        assert_eq!(nucleus_tone_target(&['u', 'y']), Some(1));
    }

    #[test]
    fn nucleus_triphthong_uyê() {
        // uyê → tone on ê (index 2): "quyết"
        assert_eq!(nucleus_tone_target(&['u', 'y', 'ê']), Some(2));
    }

    #[test]
    fn nucleus_triphthong_oai() {
        assert_eq!(nucleus_tone_target(&['o', 'a', 'i']), Some(1));
    }

    #[test]
    fn nucleus_illegal() {
        assert_eq!(nucleus_tone_target(&['e', 'l']), None);
        assert_eq!(nucleus_tone_target(&['a', 'l']), None);
        assert_eq!(nucleus_tone_target(&[]), None);
    }

    #[test]
    fn onset_prefix_valid() {
        assert!(is_onset_prefix(b"n"));
        assert!(is_onset_prefix(b"ng"));
        assert!(is_onset_prefix(b"ngh"));
        assert!(is_onset_prefix(b"p"));
        assert!(is_onset_prefix(b"ph"));
    }

    #[test]
    fn onset_prefix_invalid() {
        assert!(!is_onset_prefix(b"tt"));
        assert!(!is_onset_prefix(b"bl"));
    }
}
