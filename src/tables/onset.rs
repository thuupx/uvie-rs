//! Onset (initial consonant cluster) validation.

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
pub(crate) static LEGAL_ONSETS: &[&[u8]] = &[
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
