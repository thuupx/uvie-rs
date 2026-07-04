//! Double-vowel circumflex target lookup (used by the composing layer to detect
//! incomplete circumflex nuclei like `iê`, `yê`, `uê`, `â`, `ô`).

use crate::engine::UltraFastViEngine;
use crate::modes::IS_TONE_KEY;
use crate::syllable::{F_HORN, F_LITERAL};

/// Double-vowel circumflex target resolution.
pub(crate) trait DoubleVowelLookup {
    fn find_modifier_target_for_double_vowel(&self, b: u8) -> Option<usize>;
}

impl DoubleVowelLookup for UltraFastViEngine {
    #[inline]
    fn find_modifier_target_for_double_vowel(&self, b: u8) -> Option<usize> {
        if self.raw_len >= 3 {
            let prev = self.raw[self.raw_len - 2];
            let prev2 = self.raw[self.raw_len - 3];
            if self.mode.classify[prev as usize] & IS_TONE_KEY != 0 && prev2 == b {
                // Allow the tone key to sit between the two halves of an
                // incomplete circumflex nucleus:
                // - iê / yê / uê: i/y/u + e + tone + e (e.g. `ieje` -> iệ)
                // - ê:            e + tone + e         (e.g. `eje` -> ệ)
                // - â:            a + tone + a         (e.g. `aja` -> ậ)
                // - ô:            o + tone + o         (e.g. `ojo` -> ộ)
                if b == b'e' {
                    // For ê, only allow bare (e + tone + e) or glide (i/y/u + e + tone + e).
                    // Consonant onsets are NOT allowed because they conflict with
                    // English words like "reset", "telex" where the consonant
                    // between two e's is not a tone key.
                    if self.raw_len >= 4 {
                        let prev3 = self.raw[self.raw_len - 4];
                        if prev3 != b'e' && !matches!(prev3, b'i' | b'y' | b'u') {
                            return None;
                        }
                    }
                    // raw_len == 3: bare "e + tone + e" — allowed, fall through.
                } else if b != b'a' && b != b'o' {
                    return None;
                }
                // For a/o/e (bare), the doubled vowel itself is the nucleus —
                // no glide prefix required, so fall through to the normal search.
            }
        }

        let n = self.buf.len();
        for i in (0..n).rev() {
            let s = self.buf.get(i);
            if s.base == b && s.flags & F_LITERAL == 0 && s.flags & F_HORN == 0 {
                return Some(i);
            }
            if s.base == b'd' && s.flags & F_HORN != 0 {
                return None;
            }
        }
        None
    }
}
