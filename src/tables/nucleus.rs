//! Nucleus (vowel core) validation with tone-target index.

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
        seq: &['u', 'â', 'y'],
        tone_idx: 1,
    }, // uây (khuấy, nguẩy)
    NucleusEntry {
        seq: &['o', 'e', 'o'],
        tone_idx: 1,
    }, // oeo (khoèo, ngoèo)
    NucleusEntry {
        seq: &['u', 'ê', 'u'],
        tone_idx: 1,
    }, // uêu (khểu, nghễu)
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
        seq: &['o', 'ă'],
        tone_idx: 1,
    }, // oă (hoăng, loăng quăng)
    NucleusEntry {
        seq: &['ê', 'u'],
        tone_idx: 0,
    }, // êu (nếu → tone on ê)
    NucleusEntry {
        seq: &['ô', 'i'],
        tone_idx: 0,
    }, // ôi
    NucleusEntry {
        seq: &['ơ', 'i'],
        tone_idx: 0,
    }, // ơi
    NucleusEntry {
        seq: &['ư', 'a'],
        tone_idx: 0,
    }, // ưa
    NucleusEntry {
        seq: &['ư', 'i'],
        tone_idx: 0,
    }, // ưi (gửi → tone on ư)
    NucleusEntry {
        seq: &['ư', 'u'],
        tone_idx: 0,
    }, // ưu
    NucleusEntry {
        seq: &['ư', 'ơ'],
        tone_idx: 1,
    }, // ươ (hướng → tone on ơ, index 1)
    NucleusEntry {
        seq: &['u', 'ơ'],
        tone_idx: 1,
    }, // uơ (thuở, huơ — /uə/ open: horn on the o, plain u)
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

/// Returns `true` if the nucleus can be followed by a consonant coda.
///
/// In Vietnamese phonotactics, only **centering diphthongs** (where the second
/// vowel is the syllabic peak) can take a consonant coda. **Closing
/// diphthongs** (where the second vowel is an offglide: ai, ao, au, ay, eo,
/// êu, iu, oi, ôi, ơi, ui, ưi, ưu, âu, ây, etc.) and **all triphthongs** are
/// always open syllables — they cannot be followed by a coda consonant.
///
/// Centering diphthongs that CAN take a coda:
///   iê, yê, uê, uô, uo, ươ, oa, oe, oă, uâ, uy, oo
/// Plus the centering triphthong uyê (chuyển, khuyên, truyền).
///
/// Examples:
///   - iê + n = "iên" (tiên) ✓    ai + n = "ain" ✗
///   - uô + c = "uôc" (buộc) ✓    ao + c = "aoc" ✗
///   - oa + n = "oan" (khoán) ✓   au + n = "aun" ✗
///   - uy + t = "uyt" (buýt) ✓
///   - uyê + n = "uyên" (chuyển) ✓
///
/// Monophthongs (single vowels) always allow codas.
/// `oo` + coda is an engine extension for slang/colloquial (boóng, choòng).
#[inline]
pub fn nucleus_allows_coda(nucleus: &[char]) -> bool {
    if nucleus.len() <= 1 {
        return true; // Monophthongs always allow codas.
    }
    if nucleus.len() >= 3 {
        // Only the centering triphthong uyê can take a coda (chuyển, khuyên).
        // All other triphthongs (iêu, oai, ươi, uôi, etc.) are always open.
        return matches!(nucleus, ['u', 'y', 'ê']);
    }
    // Diphthongs: only centering diphthongs allow codas.
    matches!(
        nucleus,
        ['i', 'ê']
            | ['y', 'ê'] // iê, yê → tiên, yên
            | ['u', 'ê'] // uê → huênh, chuếch
            | ['u', 'ô'] // uô → buông, muốn
            | ['u', 'o'] // uo → thúong (precursor to ươ)
            | ['ư', 'ơ'] // ươ → hướng, thước
            | ['o', 'a'] // oa → khoán, hoàng
            | ['o', 'e'] // oe → khoen
            | ['o', 'ă'] // oă → hoăng
            | ['u', 'â'] // uâ → chuẩn, khuất
            | ['u', 'y'] // uy → buýt, khuyên
            | ['o', 'o'] // oo → boóng (engine extension)
    )
}

/// Coda class for rhyme compatibility (teen-code shorthands normalized).
#[derive(PartialEq, Eq, Clone, Copy)]
enum CodaClass {
    M,
    N,
    Ng,
    Nh,
    Ch,
    P,
    T,
    C,
}

fn classify_coda(coda: &[u8]) -> Option<CodaClass> {
    match coda {
        b"m" => Some(CodaClass::M),
        b"n" => Some(CodaClass::N),
        b"ng" | b"g" => Some(CodaClass::Ng),
        b"nh" | b"h" | b"nk" => Some(CodaClass::Nh),
        b"ch" => Some(CodaClass::Ch),
        b"p" => Some(CodaClass::P),
        b"t" => Some(CodaClass::T),
        b"c" | b"k" => Some(CodaClass::C),
        // Vowel off-glide codas (i/y/u/o) are handled by the nucleus table;
        // accept them defensively.
        b"i" | b"y" | b"u" | b"o" => Some(CodaClass::N),
        _ => None,
    }
}

/// Rhyme-level (nucleus + coda) compatibility per the standard Quốc Ngữ rime
/// inventory.
///
/// Stricter than [`nucleus_allows_coda`]: also encodes the per-vowel coda
/// restrictions that the coarse centering/closing split misses. Without this,
/// tone keys inside English words produce fake Vietnamese such as
/// `bings`→`bíng`, `thecs`→`théc`, `sowrng`→`sởng`.
///
/// Monophthong coda sets (standard orthography):
///   - `a`: m n ng nh p t c ch (`anh`/`ach` use letter a for /ă/)
///   - `ă â e o ô u`: m n ng p t c
///   - `ê i y`: m n nh p t ch (no velar ng/c — "inh" not "ing")
///   - `ơ`: m n p t only (no velar/palatal)
///   - `ư`: m n ng t c (no p, no palatal)
///
/// Diphthong coda sets (centering only, per the standard rime table):
///   - `iê yê`: m n ng p t c · `uô`: m n ng t c · `ươ`: m n ng p t c
///   - `uâ`: m n ng t c · `uê`: m n nh p t ch · `uy`: n nh p t ch
///   - `oa`: all eight · `oe`: m n p t c · `oă`: m n ng t c
///   - `uo` (/uə/ transient): m n ng t c · `uơ`: open only
///   - `oo` (engine extension): any consonant coda (boóng, choòng)
///
/// In `relaxed` mode the lone `g`/`h` coda shorthands bypass this check
/// entirely (teen code, rendered verbatim — e.g. "đặh").
pub fn rhyme_coda_compatible(nucleus: &[char], coda: &[u8], relaxed: bool) -> bool {
    if coda.is_empty() {
        return true;
    }
    // Teen-code shorthands (relaxed mode): rendered verbatim, no rhyme
    // restriction (e.g. "đawjh" → "đặh").
    if relaxed && matches!(coda, b"g" | b"h") {
        return true;
    }
    let Some(class) = classify_coda(coda) else {
        return false;
    };
    use CodaClass::*;
    match nucleus {
        ['a'] => true,
        ['ă'] | ['â'] | ['ô'] => matches!(class, M | N | Ng | P | T | C),
        // e/o/u: the plain vowel is a transient for ê/ô/ư (oo/w transforms),
        // and c is a transient for ch — accept the intermediate states. [e]+ch
        // is also a growth intermediate (e→ê via the second e: "lech"+"e" →
        // "lệch"), unlike [o]+ch / [u]+ch whose transforms dead-end.
        ['e'] => matches!(class, M | N | Ng | P | T | C | Ch),
        ['o'] | ['u'] => matches!(class, M | N | Ng | P | T | C),
        ['ê'] => true,
        ['i'] | ['y'] => matches!(class, M | N | Nh | P | T | Ch | C),
        ['ơ'] => matches!(class, M | N | P | T),
        ['ư'] => matches!(class, M | N | Ng | T | C),
        // Centering diphthongs.
        ['i', 'ê'] | ['y', 'ê'] => matches!(class, M | N | Ng | P | T | C),
        ['u', 'ô'] => matches!(class, M | N | Ng | P | T | C),
        ['ư', 'ơ'] => matches!(class, M | N | Ng | P | T | C),
        ['u', 'â'] => matches!(class, M | N | Ng | T | C),
        ['u', 'ê'] => matches!(class, M | N | Nh | P | T | Ch | C),
        ['u', 'y'] => matches!(class, N | Nh | P | T | Ch | C),
        ['o', 'a'] => matches!(class, M | N | Ng | Nh | P | T | C | Ch),
        ['o', 'e'] => matches!(class, M | N | P | T | C),
        ['o', 'ă'] => matches!(class, M | N | Ng | T | C),
        // /uə/ rhyme (thuốc, muốn, buông) — transient plain-o form; the toned
        // form resolves to ['u','ô'] via apply_coda_tone_rule.
        ['u', 'o'] => matches!(class, M | N | Ng | T | C),
        // /uə/ open (thuở) is written with the horn; closed takes uô.
        ['u', 'ơ'] => false,
        // oo engine extension (boóng, choòng) — keep fully permissive.
        ['o', 'o'] => true,
        // uyê triphthong (chuyên, quyết).
        ['u', 'y', 'ê'] => matches!(class, N | T),
        // Closing diphthongs and all other triphthongs are always open.
        _ => false,
    }
}
