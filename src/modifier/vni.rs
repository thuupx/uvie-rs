//! VNI-specific modifier keys: `6` (circumflex), `7` (horn), `8` (breve), `9` (đ).

use crate::engine::UltraFastViEngine;
use crate::modes::IS_VOWEL;
use crate::syllable::{F_CAPS, F_CIRCUMFLEX, F_HORN, F_LITERAL, Syl};
use crate::tone_handler::ToneHandler;

/// VNI-mode modifier handling (`6` → circumflex, `7` → horn, `8` → breve, `9` → đ).
pub(crate) trait VniModifier {
    fn handle_vni_6(&mut self, caps: bool);
    fn handle_vni_7(&mut self, caps: bool);
    fn handle_vni_8(&mut self, caps: bool);
    fn handle_vni_9(&mut self, caps: bool);
}

impl VniModifier for UltraFastViEngine {
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
}
