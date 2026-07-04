//! Modifier key handling (Telex w/d, VNI 6/7/8/9, double-vowel circumflex).

mod double;
mod telex;
mod vni;

use crate::engine::UltraFastViEngine;
use crate::syllable::Syl;

pub(crate) use double::DoubleVowelLookup;
pub(crate) use telex::TelexModifier;
pub(crate) use vni::VniModifier;

/// Modifier key dispatcher (circumflex, horn, breve, đ).
pub(crate) trait ModifierHandler {
    fn handle_modifier(&mut self, b: u8, caps: bool);
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
}
