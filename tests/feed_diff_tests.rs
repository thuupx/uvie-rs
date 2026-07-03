//! Integration tests for UltraFastViEngine::feed_diff - diff-based API.

use uvie::UltraFastViEngine;
use uvie::diff::Diffable;

fn apply_diff(screen: &mut String, bs: usize, suffix: &str) {
    let sc: Vec<char> = screen.chars().collect();
    *screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
    screen.push_str(suffix);
}

fn assert_diff(input: &str, expected: &str) {
    let mut engine = UltraFastViEngine::new();
    engine.set_modern_orthography(true);
    let mut screen = String::new();
    for ch in input.chars() {
        let (bs, suffix) = engine.feed_diff(ch);
        apply_diff(&mut screen, bs, suffix);
    }
    assert_eq!(screen, expected, "input: {:?}", input);
}

// ------------------------------------------------------------------
// Basic words
// ------------------------------------------------------------------

#[test]
fn diff_simple_word() {
    assert_diff("xin", "xin");
}

#[test]
fn diff_tone_mark() {
    assert_diff("tooi", "tôi");
}

#[test]
fn diff_space_boundary() {
    assert_diff("xin chao", "xin chao");
}

// ------------------------------------------------------------------
// Complex syllables
// ------------------------------------------------------------------

#[test]
fn diff_diphthong_tone() {
    assert_diff("toanj", "toạn");
}

#[test]
fn diff_diphthong_ng() {
    assert_diff("thuowngj", "thượng");
}

#[test]
fn diff_medial_o() {
    assert_diff("ngoanf", "ngoàn");
}

// ------------------------------------------------------------------
// Rollback / invalid
// ------------------------------------------------------------------

#[test]
fn diff_invalid_sequence() {
    assert_diff("fgh", "fgh");
}

// ------------------------------------------------------------------
// Backspace
// ------------------------------------------------------------------

#[test]
fn diff_backspace() {
    let mut engine = UltraFastViEngine::new();
    let mut screen = String::new();

    for ch in "tooi".chars() {
        let (bs, suffix) = engine.feed_diff(ch);
        apply_diff(&mut screen, bs, suffix);
    }
    assert_eq!(screen, "tôi");

    let (bs, suffix) = engine.backspace_diff();
    apply_diff(&mut screen, bs, suffix);
    assert_eq!(screen, "tô");

    let (bs, suffix) = engine.backspace_diff();
    apply_diff(&mut screen, bs, suffix);
    assert_eq!(screen, "to");

    let (bs, suffix) = engine.backspace_diff();
    apply_diff(&mut screen, bs, suffix);
    assert_eq!(screen, "t");

    let (bs, suffix) = engine.backspace_diff();
    apply_diff(&mut screen, bs, suffix);
    assert_eq!(screen, "");
}

// ------------------------------------------------------------------
// Punctuation / word boundaries
// ------------------------------------------------------------------

#[test]
fn diff_comma_boundary() {
    assert_diff("xin,", "xin,");
}

#[test]
fn diff_period_boundary() {
    assert_diff("xin.", "xin.");
}

// ------------------------------------------------------------------
// V-C-V boundary detection
// ------------------------------------------------------------------

#[test]
fn diff_vcv_neebo() {
    let mut engine = UltraFastViEngine::new();
    let mut screen = String::new();
    for ch in "neebo".chars() {
        let (bs, suffix) = engine.feed_diff(ch);
        apply_diff(&mut screen, bs, suffix);
    }
    assert_eq!(screen, "nêbo");
    assert_eq!(engine.committed_text_diff(), "nê");
}

#[test]
fn diff_vcv_neeboo() {
    let mut engine = UltraFastViEngine::new();
    let mut screen = String::new();
    for ch in "neeboo".chars() {
        let (bs, suffix) = engine.feed_diff(ch);
        apply_diff(&mut screen, bs, suffix);
    }
    assert_eq!(screen, "nêbô");
    assert_eq!(engine.committed_text_diff(), "nê");
}

#[test]
fn diff_no_spurious_commit_tooi() {
    let mut engine = UltraFastViEngine::new();
    let mut screen = String::new();
    for ch in "tooi".chars() {
        let (bs, suffix) = engine.feed_diff(ch);
        apply_diff(&mut screen, bs, suffix);
    }
    assert_eq!(screen, "tôi");
    assert_eq!(engine.committed_text_diff(), "");
}

#[test]
fn diff_fast_typing_ddau() {
    assert_diff("ddaau", "đâu");
}

// ------------------------------------------------------------------
// Backspace + retype scenarios
// ------------------------------------------------------------------

#[test]
fn diff_backspace_retype_thajta() {
    let mut engine = UltraFastViEngine::new();
    let mut screen = String::new();

    for ch in "thajta".chars() {
        let (bs, suffix) = engine.feed_diff(ch);
        apply_diff(&mut screen, bs, suffix);
    }
    assert_eq!(screen, "thật");

    // BS once
    let (bs, suffix) = engine.backspace_diff();
    apply_diff(&mut screen, bs, suffix);
    assert_eq!(screen, "thạt");

    // Type 'a' again
    let (bs, suffix) = engine.feed_diff('a');
    apply_diff(&mut screen, bs, suffix);
    assert_eq!(screen, "thật");
}

#[test]
fn diff_backspace_clear_retype() {
    let mut engine = UltraFastViEngine::new();
    let mut screen = String::new();

    for ch in "thajta".chars() {
        let (bs, suffix) = engine.feed_diff(ch);
        apply_diff(&mut screen, bs, suffix);
    }
    assert_eq!(screen, "thật");

    // BS 6 times to clear
    for _ in 0..6 {
        let (bs, suffix) = engine.backspace_diff();
        apply_diff(&mut screen, bs, suffix);
    }
    assert_eq!(screen, "");

    // Retype fresh
    for ch in "thajta".chars() {
        let (bs, suffix) = engine.feed_diff(ch);
        apply_diff(&mut screen, bs, suffix);
    }
    assert_eq!(screen, "thật");
}

#[test]
fn diff_sticky_after_backspace() {
    let mut engine = UltraFastViEngine::new();
    let mut screen = String::new();

    for ch in "thajt".chars() {
        let (bs, suffix) = engine.feed_diff(ch);
        apply_diff(&mut screen, bs, suffix);
    }

    let (bs, suffix) = engine.feed_diff('a');
    apply_diff(&mut screen, bs, suffix);
    assert_eq!(screen, "thật");

    // BS once -> "thạt"
    let (bs, suffix) = engine.backspace_diff();
    apply_diff(&mut screen, bs, suffix);
    assert_eq!(screen, "thạt");

    // Type 'a' again -> "thật"
    let (bs, suffix) = engine.feed_diff('a');
    apply_diff(&mut screen, bs, suffix);
    assert_eq!(screen, "thật");
}

// ------------------------------------------------------------------
// gif / tim sequences
// ------------------------------------------------------------------

#[test]
fn diff_gif() {
    assert_diff("gif", "gì");
}

#[test]
fn diff_tim_sequences() {
    assert_diff("timf", "tìm");
    assert_diff("tim", "tim");
}

// ------------------------------------------------------------------
// Multiple syllables
// ------------------------------------------------------------------

#[test]
fn diff_boo_boo() {
    assert_diff("boo boo", "bô bô");
}

// ------------------------------------------------------------------
// English passthrough
// ------------------------------------------------------------------

#[test]
fn diff_english_clear() {
    assert_diff("clear", "clear");
}

#[test]
fn diff_english_blob() {
    assert_diff("blob", "blob");
}

// ------------------------------------------------------------------
// Double-cancel
// ------------------------------------------------------------------

#[test]
fn diff_double_tone_cancel() {
    assert_diff("tess", "tes");
    assert_diff("teff", "tef");
}

#[test]
fn diff_double_w_cancel() {
    assert_diff("showw", "show");
    assert_diff("oww", "ow");
}

// ------------------------------------------------------------------
// dd/ww cancel
// ------------------------------------------------------------------

#[test]
fn diff_dd_cancel() {
    assert_diff("ddd", "dd");
}

#[test]
fn diff_ww_cancel() {
    assert_diff("ww", "w");
    assert_diff("wwork", "work");
}
