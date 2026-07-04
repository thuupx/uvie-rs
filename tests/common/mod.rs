//! Shared helpers for integration tests.

use uvie::{InputMethod, UltraFastViEngine};

/// Simulates IME typing: whitespace commits the current composing word,
/// and the final result includes committed text + any remaining composing text.
pub fn type_seq(engine: &mut UltraFastViEngine, seq: &str) -> String {
    let mut result = String::new();
    for c in seq.chars() {
        if c.is_whitespace() {
            result.push_str(engine.current_composing());
            engine.commit();
            result.push(c);
        } else {
            engine.feed(c);
        }
    }
    result.push_str(engine.current_composing());
    result
}

pub fn type_seq_vni(seq: &str) -> String {
    let mut e = UltraFastViEngine::new();
    e.set_input_method(InputMethod::Vni);
    type_seq(&mut e, seq)
}
