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
    // "ău" is NOT a standard Vietnamese rime (the [ău] sound is written "au",
    // e.g. "sau", "lau"; "tầu" is a nonstandard variant of "tàu"). The breve
    // cannot apply here, so the w is restored and the word passes through.
    let mut e = UltraFastViEngine::new();
    let result = type_seq(&mut e, "tawuf");
    assert_eq!(
        result, "tawuf",
        "ău is not a legal rime; passthrough expected"
    );
}

#[test]
fn test_nucleus_io() {
    // "io" is not a legal Vietnamese nucleus (no word in the 22k list uses it),
    // so "kio" passes through unchanged.
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "kio"), "kio", "kio should produce kio");
}

#[test]
fn test_nucleus_eo_circumflex() {
    // "êo" is NOT a standard Vietnamese rime (ê only combines with u: "êu").
    // "keeo" must pass through raw instead of rendering the fake word "kêo".
    // This also covers "theeo" → "theeo" (not "thêo").
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "keeo"),
        "keeo",
        "êo is not a legal rime; passthrough expected"
    );
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

// ========== UPPERCASE ƯƠ BUG FIX TESTS ==========
// Bug: typing NGUOWCJ produced "NGỰOC" instead of "NGƯỢC".
// Root cause: in try_apply_w_non_cancel, the "uo"→"ươ" promotion checked
// `prev.flags == 0`, which failed when the 'u' carried F_CAPS (uppercase).

#[test]
fn test_uppercase_uo_horn_promotion() {
    // NGUOWCJ → NGƯỢC (ngược in uppercase)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "NGUOWCJ"),
        "NGƯỢC",
        "NGUOWCJ should produce NGƯỢC"
    );
}

#[test]
fn test_lowercase_uo_horn_promotion_still_works() {
    // nguowcj → ngược (regression guard for the lowercase path)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "nguowcj"),
        "ngược",
        "nguowcj should produce ngược"
    );
}

#[test]
fn test_mixed_case_uo_horn_promotion() {
    // Mixed case: nguOwcj → ngƯợc? The horn promotion should still fire
    // because F_CAPS on the 'u' no longer blocks it.
    let mut e = UltraFastViEngine::new();
    let result = type_seq(&mut e, "nguOwcj");
    assert!(
        result.contains('Ư') || result.contains('ư'),
        "nguOwcj should still apply horn to u, got {}",
        result
    );
}

// ========== REVERSE TYPING ORDER: uw + o → ươ ==========
// Bug: typing "nguwocs" produced "ngứoc" instead of "ngước".
// Root cause: after "uw" produces "ư", typing 'o' did not auto-promote to
// 'ơ' to form the "ươ" diphthong. The engine only handled the "uo" + w
// order, not the "uw" + o order.

#[test]
fn test_uw_then_o_forms_uo_horn() {
    // nguwocs → ngước (sắc tone on ơ)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "nguwocs"),
        "ngước",
        "nguwocs should produce ngước"
    );
}

#[test]
fn test_uw_then_o_with_nang_tone() {
    // nguwocj → ngược (nặng tone on ơ)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "nguwocj"),
        "ngược",
        "nguwocj should produce ngược"
    );
}

#[test]
fn test_uw_then_o_no_tone() {
    // nguwoc → ngươc (no tone, just horn diphthong + coda)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "nguwoc"),
        "ngươc",
        "nguwoc should produce ngươc"
    );
}

#[test]
fn test_uw_then_o_open_syllable() {
    // nguwo → ngươ (open syllable, no coda)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "nguwo"),
        "ngươ",
        "nguwo should produce ngươ"
    );
}

#[test]
fn test_uw_then_o_uppercase() {
    // NGUWOCS → NGƯỚC (uppercase, both fixes combined)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "NGUWOCS"),
        "NGƯỚC",
        "NGUWOCS should produce NGƯỚC"
    );
}

#[test]
fn test_uw_then_o_with_huyen_tone() {
    // nguwongf → ngường (huyền tone on ơ, ng coda allows all tones)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "nguwongf"),
        "ngường",
        "nguwongf should produce ngường"
    );
}

#[test]
fn test_uw_then_o_with_hoi_tone() {
    // nguwongr → ngưởng (hỏi tone on ơ, ng coda allows all tones)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "nguwongr"),
        "ngưởng",
        "nguwongr should produce ngưởng"
    );
}

#[test]
fn test_uw_then_o_with_nga_tone() {
    // nguwongx → ngưỡng (ngã tone on ơ, ng coda allows all tones)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "nguwongx"),
        "ngưỡng",
        "nguwongx should produce ngưỡng"
    );
}

#[test]
fn test_uwo_then_ng_coda() {
    // nguwong → ngương (ươ + ng coda, sắc tone)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "nguwongs"),
        "ngướng",
        "nguwongs should produce ngướng"
    );
}

#[test]
fn test_uw_then_o_uoi_triphthong() {
    // nguwois → ngưới (ươi triphthong with sắc tone)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "nguwois"),
        "ngưới",
        "nguwois should produce ngưới"
    );
}

#[test]
fn test_uw_then_o_uou_triphthong() {
    // nguwous → ngướu (ươu triphthong with sắc tone)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "nguwous"),
        "ngướu",
        "nguwous should produce ngướu"
    );
}
