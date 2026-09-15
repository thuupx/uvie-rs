//! Static helper methods used by the diff pipeline: diff computation, word
//! boundary detection, V-C-V split point, and re-rendering.

use crate::buffers::OutBuffer;
use crate::engine::UltraFastViEngine;
use crate::modes::{IS_TONE_KEY, InputMethod, Mode};

impl UltraFastViEngine {
    /// Compute minimal diff from `prev` → `new`, writing suffix into `out`.
    ///
    /// Returns `(backspaces, suffix_len)`. The caller sends `backspaces`
    /// delete keys then types the `suffix` chars to transform `prev` → `new`.
    pub(crate) fn diff_into(prev: &str, new: &str, out: &mut OutBuffer) -> (usize, usize) {
        // Byte-level common prefix first (UTF-8 is self-synchronizing: the
        // byte prefix can only end mid-char when the chars at that position
        // differ, so backing up over continuation bytes lands on the true
        // common char boundary). This avoids char-iterator overhead for the
        // dominant prefix scan.
        let pb = prev.as_bytes();
        let nb = new.as_bytes();
        let max = pb.len().min(nb.len());
        let mut common_bytes = 0usize;
        while common_bytes < max && pb[common_bytes] == nb[common_bytes] {
            common_bytes += 1;
        }
        // Back up to a char boundary (a differing multi-byte char can share
        // its lead byte with the previous char's tail). If the whole of
        // `prev` was consumed, its end is already a char boundary.
        while common_bytes > 0 && common_bytes < pb.len() && pb[common_bytes] & 0xC0 == 0x80 {
            common_bytes -= 1;
        }

        // Count prev chars: common-prefix chars + the rest, in one byte scan
        // (a char starts at every non-continuation byte).
        let mut common = 0usize;
        let mut prev_count = 0usize;
        for (i, &b) in pb.iter().enumerate() {
            if b & 0xC0 != 0x80 {
                prev_count += 1;
                if i < common_bytes {
                    common += 1;
                }
            }
        }
        let backspaces = prev_count - common;
        out.clear();
        let _ = out.push_str(&new[common_bytes..]);
        (backspaces, out.len())
    }

    /// Returns true for characters that end the current composing word.
    ///
    /// Any ASCII non-alphanumeric character is a word boundary — this covers
    /// `/`, `\`, `-`, `_`, `@`, `#`, etc. that users type mid-sentence
    /// (e.g. URLs, paths, code). Without this, a leading `/` would be pushed
    /// into the buffer as a literal consonant, corrupting `is_legal_onset`
    /// and silently disabling tone/modifier application for the rest of the
    /// word (see bug #11: `/duowcs` produced `/duowcs` instead of `/được`).
    ///
    /// Digits are NOT boundaries because VNI uses `0-9` as tone/modifier keys.
    /// Non-ASCII characters (incl. precomposed Vietnamese) are NOT boundaries
    /// here — they are decomposed by `feed()` and flow through the normal path.
    /// Unicode whitespace is still a boundary via `is_whitespace()`.
    #[inline]
    pub(crate) fn is_word_boundary(ch: char) -> bool {
        ch.is_whitespace() || (ch.is_ascii() && !ch.is_ascii_alphanumeric())
    }

    /// Returns true if the composed output equals the raw input (no Vietnamese transforms).
    ///
    /// `raw` holds raw keystrokes, which are always ASCII, so a byte-length
    /// mismatch immediately rules out passthrough (any Vietnamese render is
    /// longer in bytes). Only the equal-length case falls through to a
    /// byte compare — no char-iterator overhead on either path.
    #[inline]
    pub(crate) fn is_raw_passthrough_slice(raw: &[char], composed: &str) -> bool {
        if raw.is_empty() {
            return true;
        }
        let nb = composed.as_bytes();
        if nb.len() != raw.len() {
            // raw is pure ASCII, so equal char counts imply equal byte counts
            // for a passthrough render — a longer/shorter byte length means
            // at least one multi-byte (Vietnamese) char: not passthrough.
            return false;
        }
        for (i, &r) in raw.iter().enumerate() {
            if nb[i] != r as u8 {
                return false;
            }
        }
        true
    }

    /// Find the V-C-V split point: index in raw_chars where the second syllable starts.
    pub(crate) fn find_split_point(raw: &[char]) -> usize {
        let n = raw.len();
        if n == 0 {
            return 0;
        }
        let new_vowel_pos = n - 1;
        let mut last_old_vowel = 0usize;
        let mut found_old_vowel = false;
        for i in (0..new_vowel_pos).rev() {
            if Self::is_ascii_vowel(raw[i] as u8) {
                last_old_vowel = i;
                found_old_vowel = true;
                break;
            }
        }
        if !found_old_vowel {
            return 0;
        }
        if last_old_vowel < new_vowel_pos {
            let first_cons_after_vowel = (last_old_vowel + 1..new_vowel_pos)
                .find(|&i| !Self::is_ascii_vowel(raw[i] as u8))
                .unwrap_or(new_vowel_pos);
            return first_cons_after_vowel;
        }
        0
    }

    /// Re-render a slice of chars through the engine and return rendered output.
    ///
    /// On `std`: uses a thread-local scratch engine to avoid allocating a
    /// fresh `UltraFastViEngine` on every V-C-V split.
    /// On `no_std`: creates a new engine each time (no `thread_local!`).
    ///
    /// Sets both `mode` AND `input_method` on the scratch engine so that
    /// `decompose_vietnamese_char` (which uses `input_method`) works correctly
    /// for VNI precomposed input.
    pub(crate) fn rerender_chars(
        raw: &[char],
        mode: &'static Mode,
        is_simple_telex: bool,
    ) -> OutBuffer {
        #[cfg(feature = "std")]
        {
            thread_local! {
                static SCRATCH: std::cell::RefCell<UltraFastViEngine> =
                    std::cell::RefCell::new(UltraFastViEngine::new());
            }
            SCRATCH.with(|s| {
                let mut eng = s.borrow_mut();
                eng.clear();
                eng.mode = mode;
                // Sync input_method with the mode so decompose_vietnamese_char
                // uses the correct Telex/VNI key mappings.
                eng.input_method = match mode.resolver {
                    crate::modes::ResolverKind::Telex => InputMethod::Telex,
                    crate::modes::ResolverKind::Vni => InputMethod::Vni,
                };
                eng.is_simple_telex = is_simple_telex;
                for &c in raw {
                    eng.feed(c);
                }
                eng.out_buf.clone()
            })
        }
        #[cfg(not(feature = "std"))]
        {
            let mut eng = UltraFastViEngine::new();
            eng.mode = mode;
            eng.input_method = match mode.resolver {
                crate::modes::ResolverKind::Telex => InputMethod::Telex,
                crate::modes::ResolverKind::Vni => InputMethod::Vni,
            };
            eng.is_simple_telex = is_simple_telex;
            for &c in raw {
                eng.feed(c);
            }
            eng.out_buf.clone()
        }
    }

    #[inline]
    pub(crate) fn is_ascii_vowel(b: u8) -> bool {
        matches!(b, b'a' | b'e' | b'i' | b'o' | b'u' | b'y')
    }

    #[inline]
    pub(crate) fn is_tone_key_in_mode(ch: char, mode: &Mode) -> bool {
        let b = ch as u8;
        mode.classify[b as usize] & IS_TONE_KEY != 0
    }

    #[inline]
    pub(crate) fn is_single_consonant_appended_slice(
        raw: &[char],
        last_valid_raw_len: usize,
    ) -> bool {
        if raw.len() != last_valid_raw_len + 1 {
            return false;
        }
        let ch = raw[last_valid_raw_len];
        !Self::is_ascii_vowel(ch as u8)
    }

    /// Find the raw index where the coda starts (index past the last vowel).
    /// If there is no vowel, the whole slice is treated as onset/coda.
    #[inline]
    pub(crate) fn raw_coda_start(raw: &[char]) -> usize {
        let mut last_vowel = None;
        for (i, &c) in raw.iter().enumerate() {
            if Self::is_ascii_vowel(c as u8) {
                last_vowel = Some(i);
            }
        }
        last_vowel.map(|i| i + 1).unwrap_or(0)
    }
}
