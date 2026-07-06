//! Telex-specific modifier keys: `w` (horn/ư) and `d` (đ).

use crate::engine::UltraFastViEngine;
use crate::modes::IS_VOWEL;
use crate::syllable::{F_CAPS, F_CIRCUMFLEX, F_HORN, F_LITERAL, Syl};
use crate::tone_handler::ToneHandler;
use crate::validation::SyllableValidator;

/// Telex-mode modifier handling (`w` → horn/ư, `d` → đ).
pub(crate) trait TelexModifier {
    fn handle_telex_w(&mut self, caps: bool);
    fn handle_telex_d(&mut self, caps: bool);
    fn try_apply_w_non_cancel(&mut self, idx: usize, nucleus_start: usize, caps: bool) -> bool;
}

impl TelexModifier for UltraFastViEngine {
    #[inline]
    fn handle_telex_w(&mut self, caps: bool) {
        let n = self.buf.len();

        // Find nucleus boundaries so w can modify vowels even with coda present
        let (_onset_end, nucleus_start, nucleus_end, _coda_start) = self.partition_syllable();

        // Collect w-target candidates (u, o, a) in the nucleus, in backwards order.
        let mut candidates = [0usize; 24];
        let mut candidate_count = 0usize;
        for i in (nucleus_start..nucleus_end).rev() {
            let syl = self.buf.get(i);
            if matches!(syl.base, b'u' | b'o' | b'a') {
                candidates[candidate_count] = i;
                candidate_count += 1;
            }
        }

        // First pass: try each candidate. If a target produces a valid Vietnamese
        // syllable, keep it. This fixes cases like "chuaw" -> "chưa" where the
        // backwards-first heuristic would otherwise modify the wrong vowel.
        //
        // Optimization: instead of cloning the entire SylBuf (192 bytes) for
        // each candidate, we snapshot only the 2 entries that could change
        // (the target + the tone carrier) and restore them on failure.
        for idx in 0..candidate_count {
            let i = candidates[idx];
            // Snapshot the target and its neighbor (reapply_tone may touch
            // a different index).
            let snap_i = *self.buf.get(i);
            let snap_len = self.buf.len();
            if self.try_apply_w_non_cancel(i, nucleus_start, caps) && self.is_valid_vietnamese() {
                return;
            }
            // Restore: undo any changes made by try_apply_w_non_cancel.
            self.buf.set(i, snap_i);
            while self.buf.len() > snap_len {
                self.buf.pop();
            }
        }

        // No valid candidate found. If there is only a single candidate and it
        // can be applied (i.e. it does not already carry F_HORN), preserve the
        // original behaviour by applying it even if the result is invalid (so
        // double-w cancellation works for e.g. "showw" -> "show").
        if candidate_count == 1 {
            let i = candidates[0];
            let snap_i = *self.buf.get(i);
            let snap_len = self.buf.len();
            if self.try_apply_w_non_cancel(i, nucleus_start, caps) {
                return;
            }
            self.buf.set(i, snap_i);
            while self.buf.len() > snap_len {
                self.buf.pop();
            }
        }

        // Second pass: cancellation (target already has F_HORN, or existing 'w').
        for i in (nucleus_start..nucleus_end).rev() {
            let syl = self.buf.get(i);
            match syl.base {
                b'u' => {
                    if self.is_u_glide(i) {
                        continue;
                    }
                    if i > nucleus_start && self.buf.get(i - 1).base == b'u' {
                        continue;
                    }
                    if syl.flags & F_HORN != 0 {
                        let reverted = Syl::literal(syl.base, syl.flags & F_CAPS != 0);
                        self.buf.set(i, reverted);
                        if self.raw_len > 0 {
                            self.raw_len -= 1;
                        }
                        self.buf.push(Syl::literal(b'w', caps));
                        self.reapply_tone_after_nucleus_change();
                        return;
                    }
                }
                b'o' => {
                    if syl.flags & F_HORN != 0 {
                        let reverted = Syl::literal(syl.base, syl.flags & F_CAPS != 0);
                        self.buf.set(i, reverted);
                        if self.raw_len > 0 {
                            self.raw_len -= 1;
                        }
                        self.buf.push(Syl::literal(b'w', caps));
                        if i > 0 {
                            let prev = self.buf.get(i - 1);
                            if prev.base == b'u' && prev.flags & F_HORN != 0 {
                                let reverted_u = Syl::literal(b'u', prev.flags & F_CAPS != 0);
                                self.buf.set(i - 1, reverted_u);
                            }
                        }
                        self.reapply_tone_after_nucleus_change();
                        return;
                    }
                }
                b'a' => {
                    if syl.flags & F_CIRCUMFLEX != 0 {
                        continue;
                    }
                    if syl.flags & F_HORN != 0 {
                        let reverted = Syl::literal(syl.base, syl.flags & F_CAPS != 0);
                        self.buf.set(i, reverted);
                        if self.raw_len > 0 {
                            self.raw_len -= 1;
                        }
                        self.buf.push(Syl::literal(b'w', caps));
                        self.reapply_tone_after_nucleus_change();
                        return;
                    }
                }
                _ => {}
            }
        }

        // Third pass: look for existing 'w' with F_HORN (for cancellation)
        for i in (0..n).rev() {
            let syl = self.buf.get(i);
            if syl.base == b'w' && syl.flags & F_HORN != 0 {
                let reverted = Syl::literal(b'w', syl.flags & F_CAPS != 0);
                self.buf.set(i, reverted);
                if self.raw_len > 0 {
                    self.raw_len -= 1;
                }
                self.buf.push(Syl::literal(b'w', caps));
                self.reapply_tone_after_nucleus_change();
                return;
            }
            // Stop searching when we hit consonants after checking w-cancellation
            if self.mode.classify[syl.base as usize] & IS_VOWEL == 0 && syl.base != b'w' {
                break;
            }
        }

        // No match - standalone 'w' becomes ư at onset.
        let onset_len = self.onset_len();
        if onset_len == n {
            let mut syl = Syl::literal(b'w', caps);
            syl.flags |= F_HORN;
            syl.out = if caps { 'Ư' } else { 'ư' };
            self.buf.push(syl);
            self.reapply_tone_after_nucleus_change();
        } else {
            self.buf.push(Syl::literal(b'w', caps));
        }
    }

    /// Apply the non-cancelling w modifier to a single nucleus target and return
    /// true if the modification was applied. Does not touch candidates that
    /// already carry F_HORN (those are handled in the cancellation pass).
    fn try_apply_w_non_cancel(&mut self, idx: usize, nucleus_start: usize, _caps: bool) -> bool {
        let syl = self.buf.get(idx);
        if syl.flags & F_HORN != 0 {
            return false;
        }
        match syl.base {
            b'u' => {
                if self.is_u_glide(idx) {
                    return false;
                }
                // Skip the second 'u' in a consecutive "uu" inside the nucleus.
                if idx > nucleus_start && self.buf.get(idx - 1).base == b'u' {
                    return false;
                }
                let updated = self.buf.get(idx).clone().with_horn();
                self.buf.set(idx, updated);
                self.reapply_tone_after_nucleus_change();
                true
            }
            b'o' => {
                let updated = self.buf.get(idx).clone().with_horn();
                self.buf.set(idx, updated);
                if idx > 0 && idx > nucleus_start {
                    let prev = self.buf.get(idx - 1);
                    // Promote a preceding plain 'u' to 'ư' when 'o' receives a
                    // horn, forming the "ươ" diphthong (e.g. "nguow" → "ngươ").
                    // Only check that no horn/circumflex is already set; F_CAPS
                    // (uppercase) must NOT block the promotion, otherwise
                    // "NGUOWCJ" produces "NGỰOC" instead of "NGƯỢC".
                    if prev.base == b'u'
                        && prev.flags & (F_HORN | F_CIRCUMFLEX) == 0
                        && !self.is_u_glide(idx - 1)
                    {
                        let promoted = prev.clone().with_horn();
                        self.buf.set(idx - 1, promoted);
                    }
                }
                self.reapply_tone_after_nucleus_change();
                true
            }
            b'a' => {
                if syl.flags & F_CIRCUMFLEX != 0 {
                    return false;
                }
                let updated = self.buf.get(idx).clone().with_horn();
                self.buf.set(idx, updated);
                self.reapply_tone_after_nucleus_change();
                true
            }
            _ => false,
        }
    }

    #[inline]
    fn handle_telex_d(&mut self, caps: bool) {
        let n = self.buf.len();

        for i in (0..n).rev() {
            let s = self.buf.get(i);
            if s.base == b'd' && s.flags & F_HORN != 0 {
                if self.is_valid_vietnamese() {
                    let reverted = Syl::literal(b'd', s.flags & F_CAPS != 0);
                    self.buf.set(i, reverted);
                    self.buf.push(Syl::literal(b'd', caps));
                    if self.raw_len > 0 {
                        self.raw_len -= 1;
                    }
                    self.mark_all_literal();
                    return;
                }
                break;
            }
            if s.base == b'd' && s.flags & F_LITERAL == 0 && s.flags & F_HORN == 0 {
                let is_in_onset = (0..i).all(|j| {
                    let sj = self.buf.get(j);
                    self.mode.classify[sj.base as usize] & IS_VOWEL == 0 && sj.base != b'w'
                });
                if !is_in_onset {
                    break;
                }
                let new_syl = Syl {
                    base: b'd',
                    out: if s.flags & F_CAPS != 0 { 'Đ' } else { 'đ' },
                    tone: 0,
                    flags: s.flags | F_HORN,
                };
                self.buf.set(i, new_syl);
                return;
            }
        }

        self.buf.push(Syl::literal(b'd', caps));
    }
}
