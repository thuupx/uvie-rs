//! Core composing logic: keystroke processing, vowel handling, and rendering.

use crate::engine::UltraFastViEngine;
use crate::modes::{IS_MODIFIER, IS_TONE_KEY, IS_VOWEL};
use crate::modifier::ModifierHandler;
use crate::syllable::{F_CAPS, F_CIRCUMFLEX, F_LITERAL, F_TONE_SET, Syl};
use crate::tone_handler::ToneHandler;
use crate::validation::SyllableValidator;

/// Core composing engine: classify and handle keystrokes, render output.
pub(crate) trait Composable {
    fn process_key(&mut self, b: u8, caps: bool);
    fn handle_consonant(&mut self, b: u8, caps: bool);
    fn handle_vowel(&mut self, b: u8, caps: bool);
    fn push_raw_key(&mut self, b: u8, caps: bool);
    fn render_out_buf(&mut self);
    fn render_passthrough(&mut self);
}

impl UltraFastViEngine {
    /// Support tones typed between the two vowels of an incomplete iê/yê/uê
    /// nucleus. For example in Telex: `ieje` (i-e-j-e) should produce `iệ`,
    /// `yefe` (y-e-f-e) should produce `yề`, `ueje` (u-e-j-e) should produce `uệ`.
    #[inline]
    fn apply_mid_nucleus_tone(&mut self, b: u8) {
        if b != b'e' {
            return;
        }
        let rl = self.raw_len;
        if rl < 4 {
            return;
        }
        let first = self.raw[rl - 4];
        let mid = self.raw[rl - 3];
        let tone_key = self.raw[rl - 2];
        let last = self.raw[rl - 1];
        if mid != b'e' || last != b'e' {
            return;
        }
        if !matches!(first, b'i' | b'y' | b'u') {
            return;
        }
        let tone_val = self.mode.tone[tone_key as usize];
        if tone_val == 0 {
            return;
        }
        // The literal tone key should be the last buffer entry because the
        // second 'e' modified the previous vowel in place.
        let Some(tone_syl) = self.buf.pop() else {
            return;
        };
        if tone_syl.base != tone_key || tone_syl.flags & F_TONE_SET != 0 {
            self.buf.push(tone_syl);
            return;
        }
        if let Some(carrier) = self.tone_carrier_idx() {
            let s = self.buf.get_mut(carrier);
            s.tone = tone_val;
            s.flags |= F_TONE_SET;
            s.recompute_out();
        } else {
            self.buf.push(tone_syl);
        }
    }
}

impl Composable for UltraFastViEngine {
    #[inline]
    fn process_key(&mut self, b: u8, caps: bool) {
        let attr = self.mode.classify[b as usize];

        if attr & IS_TONE_KEY != 0 {
            self.handle_tone_key(b, caps);
        } else if attr & IS_MODIFIER != 0 {
            self.handle_modifier(b, caps);
        } else if attr & IS_VOWEL != 0 {
            self.handle_vowel(b, caps);
        } else {
            self.handle_consonant(b, caps);
        }
    }

    #[inline]
    fn handle_consonant(&mut self, b: u8, caps: bool) {
        self.buf.push(Syl::literal(b, caps));
    }

    #[inline]
    fn handle_vowel(&mut self, b: u8, caps: bool) {
        // Check for double-vowel modifier (aa→â, ee→ê, oo→ô).
        if matches!(b, b'a' | b'e' | b'o') {
            if let Some(target_idx) = self.find_modifier_target_for_double_vowel(b) {
                let syl = self.buf.get(target_idx).clone();
                // Triple-cancel: if target already has circumflex, revert to literal.
                if syl.flags & F_CIRCUMFLEX != 0 {
                    if self.is_valid_vietnamese() {
                        let reverted = Syl::literal(syl.base, syl.flags & F_CAPS != 0);
                        self.buf.set(target_idx, reverted);
                        self.buf.push(Syl::literal(b, caps));
                        if self.raw_len > 0 {
                            self.raw_len -= 1;
                        }
                        self.mark_all_literal();
                        return;
                    }
                } else {
                    let updated = syl.with_circumflex();
                    self.buf.set(target_idx, updated);
                    self.reapply_tone_after_nucleus_change();
                    self.apply_mid_nucleus_tone(b);
                    return;
                }
            }
        }

        // Plain vowel - just push.
        self.buf.push(Syl::literal(b, caps));
        self.reapply_tone_after_nucleus_change();
    }

    #[inline]
    fn push_raw_key(&mut self, b: u8, caps: bool) {
        if self.raw_len >= 24 {
            return;
        }
        self.raw[self.raw_len] = b;
        self.raw_caps[self.raw_len] = caps;
        self.raw_len += 1;
        self.process_key(b, caps);
    }

    #[inline]
    fn render_out_buf(&mut self) {
        self.update_syl_structure();

        self.out_buf.clear();
        let n = self.buf.len();
        if n == 0 {
            return;
        }

        let has_literal = (0..n).any(|i| self.buf.get(i).flags & F_LITERAL != 0);

        if has_literal {
            self.render_passthrough();
            return;
        }

        if !self.is_valid_vietnamese() {
            self.render_passthrough();
            return;
        }

        // Valid Vietnamese - render resolved chars from buf.
        for i in 0..n {
            let s = self.buf.get(i);
            let c = s.render();
            let _ = self.out_buf.push(c);
        }
    }

    #[inline]
    fn render_passthrough(&mut self) {
        let n_buf = self.buf.len();
        let mut buf_idx = 0usize;
        let mut raw_idx = 0usize;
        while raw_idx < self.raw_len {
            let b = self.raw[raw_idx];
            let is_dh = buf_idx < n_buf
                && self.buf.get(buf_idx).base == b'd'
                && self.buf.get(buf_idx).flags & F_LITERAL == 0
                && self.buf.get(buf_idx).flags & crate::syllable::F_HORN != 0;
            if is_dh && b == b'd' && raw_idx + 1 < self.raw_len && self.raw[raw_idx + 1] == b'd' {
                let is_upper = self.buf.get(buf_idx).flags & crate::syllable::F_CAPS != 0;
                let _ = self.out_buf.push(if is_upper { 'Đ' } else { 'đ' });
                raw_idx += 2;
                buf_idx += 1;
            } else {
                let c = if self.raw_caps[raw_idx] {
                    (b as char).to_ascii_uppercase()
                } else {
                    b as char
                };
                let _ = self.out_buf.push(c);
                raw_idx += 1;
                buf_idx += 1;
            }
        }
    }
}
