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

        // First pass: look for w's modifier targets (a, o, u) in the nucleus
        // Search backwards but only within nucleus bounds
        for i in (nucleus_start..nucleus_end).rev() {
            let syl = self.buf.get(i);
            match syl.base {
                b'u' => {
                    if self.is_u_glide(i) {
                        continue;
                    }
                    // FIX: For "uuw" -> "ưu", skip 'u' that has another 'u' BEFORE it (in search direction)
                    // Since we search backwards, i-1 is the "next" char in forward direction
                    // This ensures w modifies the FIRST 'u' in a consecutive "uu" sequence
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
                    let updated = self.buf.get(i).clone().with_horn();
                    self.buf.set(i, updated);
                    self.reapply_tone_after_nucleus_change();
                    return;
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
                    let updated = self.buf.get(i).clone().with_horn();
                    self.buf.set(i, updated);
                    if i > 0 && i > nucleus_start {
                        let prev = self.buf.get(i - 1);
                        if prev.base == b'u' && prev.flags == 0 && !self.is_u_glide(i - 1) {
                            let promoted = prev.clone().with_horn();
                            self.buf.set(i - 1, promoted);
                        }
                    }
                    self.reapply_tone_after_nucleus_change();
                    return;
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
                    let updated = self.buf.get(i).clone().with_horn();
                    self.buf.set(i, updated);
                    self.reapply_tone_after_nucleus_change();
                    return;
                }
                _ => continue,
            }
        }

        // Second pass: look for existing 'w' with F_HORN (for cancellation)
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
                return None;
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
