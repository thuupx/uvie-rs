//! Modifier key handling (Telex w/d, VNI 6/7/8/9, double-vowel circumflex).

use crate::engine::UltraFastViEngine;
use crate::modes::{IS_TONE_KEY, IS_VOWEL};
use crate::syllable::{F_CAPS, F_CIRCUMFLEX, F_HORN, F_LITERAL, Syl};
use crate::tone_handler::ToneHandler;
use crate::validation::SyllableValidator;

/// Modifier key handling (circumflex, horn, breve, đ).
pub(crate) trait ModifierHandler {
    fn handle_modifier(&mut self, b: u8, caps: bool);
    fn handle_telex_w(&mut self, caps: bool);
    fn handle_telex_d(&mut self, caps: bool);
    fn handle_vni_6(&mut self, caps: bool);
    fn handle_vni_7(&mut self, caps: bool);
    fn handle_vni_8(&mut self, caps: bool);
    fn handle_vni_9(&mut self, caps: bool);
    fn find_modifier_target_for_double_vowel(&self, b: u8) -> Option<usize>;
    fn try_apply_w_non_cancel(&mut self, idx: usize, nucleus_start: usize, caps: bool) -> bool;
}

impl ModifierHandler for UltraFastViEngine {
    #[inline]
    fn handle_modifier(&mut self, b: u8, caps: bool) {
        match b {
            b'w' => self.handle_telex_w(caps),
            b'd' => self.handle_telex_d(caps),
            b'6' => self.handle_vni_6(caps),
            b'7' => self.handle_vni_7(caps),
            b'8' => self.handle_vni_8(caps),
            b'9' => self.handle_vni_9(caps),
            _ => {
                self.buf.push(Syl::literal(b, caps));
            }
        }
    }

    #[inline]
    fn handle_telex_w(&mut self, caps: bool) {
        let n = self.buf.len();

        // Find nucleus boundaries so w can modify vowels even with coda present
        let (_onset_end, nucleus_start, nucleus_end, _coda_start) = self.partition_syllable();

        let original_buf = self.buf.clone();

        // Collect w-target candidates (u, o, a) in the nucleus, in backwards order.
        // This matches the original search direction.
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
        for idx in 0..candidate_count {
            let i = candidates[idx];
            self.buf = original_buf.clone();
            if self.try_apply_w_non_cancel(i, nucleus_start, caps) && self.is_valid_vietnamese() {
                return;
            }
        }

        // No valid candidate found. If there are multiple candidates, the original
        // behaviour (apply to the first/last vowel) could be wrong, so keep the
        // buffer unchanged and fall through to cancellation / standalone handling.
        // If there is only a single candidate and it can be applied (i.e. it does not
        // already carry F_HORN), preserve the original behaviour by applying it even
        // if the result is invalid (so double-w cancellation works for e.g.
        // "showw" -> "show").
        if candidate_count == 1 {
            self.buf = original_buf.clone();
            if self.try_apply_w_non_cancel(candidates[0], nucleus_start, caps) {
                return;
            }
        }

        // Restore original buffer before the cancellation and standalone passes.
        self.buf = original_buf;

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
                    if prev.base == b'u' && prev.flags == 0 && !self.is_u_glide(idx - 1) {
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

    #[inline]
    fn handle_vni_6(&mut self, _caps: bool) {
        for i in (0..self.buf.len()).rev() {
            let syl = self.buf.get(i);
            if matches!(syl.base, b'a' | b'e' | b'o') && syl.flags & F_LITERAL == 0 {
                if syl.flags & F_CIRCUMFLEX != 0 {
                    let reverted = Syl::literal(syl.base, syl.flags & F_CAPS != 0);
                    self.buf.set(i, reverted);
                } else {
                    let updated = self.buf.get(i).clone().with_circumflex();
                    self.buf.set(i, updated);
                }
                self.reapply_tone_after_nucleus_change();
                return;
            }
            if self.mode.classify[syl.base as usize] & IS_VOWEL == 0 {
                break;
            }
        }
        self.buf.push(Syl::literal(b'6', false));
    }

    #[inline]
    fn handle_vni_7(&mut self, _caps: bool) {
        for i in (0..self.buf.len()).rev() {
            let syl = self.buf.get(i);
            if matches!(syl.base, b'o' | b'u') && syl.flags & F_LITERAL == 0 {
                if syl.flags & F_HORN != 0 {
                    let reverted = Syl::literal(syl.base, syl.flags & F_CAPS != 0);
                    self.buf.set(i, reverted);
                } else {
                    let updated = self.buf.get(i).clone().with_horn();
                    self.buf.set(i, updated);
                }
                self.reapply_tone_after_nucleus_change();
                return;
            }
            if self.mode.classify[syl.base as usize] & IS_VOWEL == 0 {
                break;
            }
        }
        self.buf.push(Syl::literal(b'7', false));
    }

    #[inline]
    fn handle_vni_8(&mut self, _caps: bool) {
        for i in (0..self.buf.len()).rev() {
            let syl = self.buf.get(i);
            if syl.base == b'a' && syl.flags & F_LITERAL == 0 {
                if syl.flags & F_HORN != 0 {
                    let reverted = Syl::literal(syl.base, syl.flags & F_CAPS != 0);
                    self.buf.set(i, reverted);
                } else {
                    let updated = self.buf.get(i).clone().with_horn();
                    self.buf.set(i, updated);
                }
                self.reapply_tone_after_nucleus_change();
                return;
            }
            if self.mode.classify[syl.base as usize] & IS_VOWEL == 0 {
                break;
            }
        }
        self.buf.push(Syl::literal(b'8', false));
    }

    #[inline]
    fn handle_vni_9(&mut self, _caps: bool) {
        for i in (0..self.buf.len()).rev() {
            let syl = self.buf.get(i);
            if syl.base == b'd' && syl.flags & F_LITERAL == 0 {
                if syl.flags & F_HORN != 0 {
                    let reverted = Syl::literal(syl.base, syl.flags & F_CAPS != 0);
                    self.buf.set(i, reverted);
                } else {
                    let new_syl = Syl {
                        base: b'd',
                        out: if syl.flags & F_CAPS != 0 { 'Đ' } else { 'đ' },
                        tone: 0,
                        flags: syl.flags | F_HORN,
                    };
                    self.buf.set(i, new_syl);
                }
                return;
            }
            if self.mode.classify[syl.base as usize] & IS_VOWEL == 0 {
                break;
            }
        }
        self.buf.push(Syl::literal(b'9', false));
    }

    #[inline]
    fn find_modifier_target_for_double_vowel(&self, b: u8) -> Option<usize> {
        if self.raw_len >= 3 {
            let prev = self.raw[self.raw_len - 2];
            let prev2 = self.raw[self.raw_len - 3];
            if self.mode.classify[prev as usize] & IS_TONE_KEY != 0 && prev2 == b {
                // Allow the tone key to sit between the two halves of an
                // incomplete iê / yê / uê nucleus (e.g. Telex `ieje` -> iệ,
                // `yefe` -> yề, `ueje` -> uệ). The pattern is i/y/u + e + tone + e.
                let prev3 = if self.raw_len >= 4 {
                    self.raw[self.raw_len - 4]
                } else {
                    0
                };
                if b != b'e' || !matches!(prev3, b'i' | b'y' | b'u') {
                    return None;
                }
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
