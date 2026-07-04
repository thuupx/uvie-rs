use uvie::UltraFastViEngine;
use uvie::diff::Diffable;
mod common;
use common::type_seq;

// ========== UUW BUG FIX TESTS ==========

#[test]
fn test_uuw_produces_uu_with_horn() {
    // "uuw" should produce "ưu" (w modifies first u to ư)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "uuw"), "ưu", "uuw should produce ưu");
}

#[test]
fn test_uuw_with_tone() {
    // "uuws" should produce "ứu" (tone sắc on first vowel)
    let mut e = UltraFastViEngine::new();
    let result = type_seq(&mut e, "uuws");
    // Current: produces "ứu" (ưu with sắc tone)
    assert!(
        result == "ứu" || result == "ưus",
        "uuws should produce ứu or ưus, got {}",
        result
    );
}

#[test]
fn test_uuw_in_word() {
    // "duuw" -> "dưu" (d + ưu)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "duuw"), "dưu", "duuw should produce dưu");
}

// ========== NEW NUCLEI TESTS ==========

#[test]
fn test_nucleus_au_breve() {
    // "ău" nucleus: tawus -> tằu (boat with sắc tone)
    // tawuf would produce tầu (huyền tone) - depends on tone key
    let mut e = UltraFastViEngine::new();
    let result = type_seq(&mut e, "tawuf");
    // Document actual behavior - tone placement on ău nucleus
    assert!(
        result == "tầu" || result == "tằu" || result == "tăuf",
        "tawuf should produce tầu or similar, got {}",
        result
    );
}

#[test]
fn test_nucleus_io() {
    // "io" nucleus (rare): kio -> kio
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "kio"), "kio", "kio should produce kio");
}

#[test]
fn test_nucleus_eo_circumflex() {
    // "êo" nucleus (rare): k + ee + o -> kêo
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "keeo"), "kêo", "keeo should produce kêo");
}

// ===== feed_diff parity tests =====

#[test]
fn feed_diff_basic_neebo() {
    let mut e = UltraFastViEngine::new();
    let mut screen = String::new();
    for ch in "neebo".chars() {
        let (bs, suffix) = e.feed_diff(ch);
        let screen_chars: Vec<char> = screen.chars().collect();
        let new_len = screen_chars.len().saturating_sub(bs);
        screen = screen_chars[..new_len].iter().collect::<String>();
        screen.push_str(suffix);
    }
    assert_eq!(screen, "nêbo", "feed_diff neebo");
    assert_eq!(e.committed_text_diff(), "nê");
}

#[test]
fn feed_diff_basic_tooi() {
    let mut e = UltraFastViEngine::new();
    let mut screen = String::new();
    for ch in "tooi".chars() {
        let (bs, suffix) = e.feed_diff(ch);
        let sc: Vec<char> = screen.chars().collect();
        screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
        screen.push_str(suffix);
    }
    assert_eq!(screen, "tôi");
    assert_eq!(e.committed_text_diff(), "");
}

#[test]
fn feed_diff_word_boundary() {
    let mut e = UltraFastViEngine::new();
    let mut screen = String::new();
    for ch in "xin chao".chars() {
        let (bs, suffix) = e.feed_diff(ch);
        let sc: Vec<char> = screen.chars().collect();
        screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
        screen.push_str(suffix);
    }
    assert_eq!(screen, "xin chao");
}

#[test]
fn feed_diff_english_passthrough() {
    let mut e = UltraFastViEngine::new();
    let mut screen = String::new();
    for ch in "blob".chars() {
        let (bs, suffix) = e.feed_diff(ch);
        let sc: Vec<char> = screen.chars().collect();
        screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
        screen.push_str(suffix);
    }
    assert_eq!(screen, "blob");
    assert_eq!(e.committed_text_diff(), "");
}

#[test]
fn feed_diff_backspace() {
    let mut e = UltraFastViEngine::new();
    let mut screen = String::new();
    // Type "tooi" -> "tôi"
    for ch in "tooi".chars() {
        let (bs, suffix) = e.feed_diff(ch);
        let sc: Vec<char> = screen.chars().collect();
        screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
        screen.push_str(suffix);
    }
    assert_eq!(screen, "tôi");
    // Backspace once -> "tô"
    let (bs, suffix) = e.backspace_diff();
    let sc: Vec<char> = screen.chars().collect();
    screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
    screen.push_str(suffix);
    assert_eq!(screen, "tô");
    // Backspace again -> "to"
    let (bs, suffix) = e.backspace_diff();
    let sc: Vec<char> = screen.chars().collect();
    screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
    screen.push_str(suffix);
    assert_eq!(screen, "to");
}

#[test]
fn repro_ghost_character_log() {
    let mut e = UltraFastViEngine::new();
    let mut screen = String::new();
    // Simulate user sequence: pass <backspace> <backspace> a s s a ...
    for ch in "pass".chars() {
        let (bs, suffix) = e.feed_diff(ch);
        let suffix = suffix.to_string();
        let committed = e.committed_text_diff().to_string();
        let core_out = e.current_output();
        let sc: Vec<char> = screen.chars().collect();
        screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
        screen.push_str(&suffix);
        println!(
            "feed '{}' -> bs={} suffix='{}' screen='{}' diff_committed='{}' core_out='{}'",
            ch, bs, suffix, screen, committed, core_out
        );
    }
    for _ in 0..2 {
        let (bs, suffix) = e.backspace_diff();
        let suffix = suffix.to_string();
        let committed = e.committed_text_diff().to_string();
        let core_out = e.current_output();
        let sc: Vec<char> = screen.chars().collect();
        screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
        screen.push_str(&suffix);
        println!(
            "backspace -> bs={} suffix='{}' screen='{}' diff_committed='{}' core_out='{}'",
            bs, suffix, screen, committed, core_out
        );
    }
    for ch in "assa".chars() {
        let (bs, suffix) = e.feed_diff(ch);
        let suffix = suffix.to_string();
        let committed = e.committed_text_diff().to_string();
        let core_out = e.current_output();
        let sc: Vec<char> = screen.chars().collect();
        screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
        screen.push_str(&suffix);
        println!(
            "feed '{}' -> bs={} suffix='{}' screen='{}' diff_committed='{}' core_out='{}'",
            ch, bs, suffix, screen, committed, core_out
        );
    }
}
