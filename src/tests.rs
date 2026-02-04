use crate::{InputMethod, UltraFastViEngine};

fn type_seq(engine: &mut UltraFastViEngine, seq: &str) -> String {
    let mut out = String::new();
    for c in seq.chars() {
        out = engine.feed(c).to_string();
    }
    out
}

fn type_seq_vni(seq: &str) -> String {
    let mut e = UltraFastViEngine::new();
    e.set_input_method(InputMethod::Vni);
    type_seq(&mut e, seq)
}

#[test]
fn telex_modifier_basic() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "aa"), "â");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "aw"), "ă");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ee"), "ê");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "oo"), "ô");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ow"), "ơ");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "uw"), "ư");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "dd"), "đ");
}

#[test]
fn tone_single_vowel_all_tones() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "as"), "á");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "af"), "à");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ar"), "ả");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ax"), "ã");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "aj"), "ạ");
}

#[test]
fn z_key_removes_tone() {
    let mut e = UltraFastViEngine::new();
    // as -> á, z -> a
    assert_eq!(type_seq(&mut e, "asz"), "a");

    let mut e = UltraFastViEngine::new();
    // az -> a
    assert_eq!(type_seq(&mut e, "az"), "a");

    let mut e = UltraFastViEngine::new();
    // axz -> a
    assert_eq!(type_seq(&mut e, "axz"), "a");
}

#[test]
fn toggling_triplet() {
    let mut e = UltraFastViEngine::new();
    // aaa -> a
    assert_eq!(type_seq(&mut e, "aaa"), "a");

    let mut e = UltraFastViEngine::new();
    // ddd -> d
    assert_eq!(type_seq(&mut e, "ddd"), "d");

    let mut e = UltraFastViEngine::new();
    // eee -> e
    assert_eq!(type_seq(&mut e, "eee"), "e");

    let mut e = UltraFastViEngine::new();
    // ooo -> o
    assert_eq!(type_seq(&mut e, "ooo"), "o");
}

#[test]
fn tone_on_modified_vowels() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "aas"), "ấ");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "awj"), "ặ");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ees"), "ế");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "oos"), "ố");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ows"), "ớ");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "uws"), "ứ");
}

#[test]
fn greedy_tone_last_wins() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "asf"), "à");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "afsj"), "ạ");
}

#[test]
fn tone_placement_two_vowels_no_coda() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "hoas"), "hoá");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "hoaf"), "hoà");
}

#[test]
fn tone_placement_two_vowels_with_coda() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "hoans"), "hoán");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "hoanj"), "hoạn");
}

#[test]
fn tone_placement_three_vowels_targets_second_vowel() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "khuya"), "khuya");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "khuyas"), "khuýa");
}

#[test]
fn whitespace_flushes_and_resets_buffer() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "aas"), "ấ");
    assert_eq!(e.feed(' '), "ấ ");
    assert_eq!(type_seq(&mut e, "as"), "á");
}

#[test]
fn tone_only_input_produces_empty() {
    let mut e = UltraFastViEngine::new();
    // First char is treated as consonant
    assert_eq!(type_seq(&mut e, "s"), "s");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "z"), "z");
}

#[test]
fn do_not_apply_to_english() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "clear"), "clear");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "flan"), "flan");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "blob"), "blob");
}

#[test]
fn special_uow_combo() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "huow"), "hươ");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "huows"), "hướ");
}

#[test]
fn valid_consonant_cluster() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "nghe"), "nghe");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "nghes"), "nghé");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "nghees"), "nghế");
}

#[test]
fn regression_qu_gi_placement() {
    // qu + a -> quá (tone on a)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "quas"), "quá");

    // qu + y -> quỳ (tone on y)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "quyf"), "quỳ");

    // qu + i -> quỉ (tone on i)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "quir"), "quỉ");

    // gi + a -> giá (tone on a)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "gias"), "giá");
}

#[test]
fn regression_vowel_pairs() {
    // oa -> hoà (tone on a, new style)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "hoaf"), "hoà");

    // oe -> hoè (tone on e, new style)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "hoef"), "hoè");

    // uy -> tuỳ (tone on y, new style)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "tuyf"), "tuỳ");

    // ia -> mía (tone on i)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "mias"), "mía");

    // ua -> múa (tone on u)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "muas"), "múa");

    // ưa -> mứa (tone on ư)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "muwas"), "mứa");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "pro"), "pro");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "free"), "free");
}

#[test]
fn regression_pho_validity() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "phos"), "phó");
}

#[test]
fn vni_basic_modifiers() {
    assert_eq!(type_seq_vni("a6"), "â");
    assert_eq!(type_seq_vni("a8"), "ă");
    assert_eq!(type_seq_vni("e6"), "ê");
    assert_eq!(type_seq_vni("o6"), "ô");
    assert_eq!(type_seq_vni("o7"), "ơ");
    assert_eq!(type_seq_vni("u7"), "ư");
    assert_eq!(type_seq_vni("d9"), "đ");
}

#[test]
fn vni_basic_tones() {
    assert_eq!(type_seq_vni("a1"), "á");
    assert_eq!(type_seq_vni("a2"), "à");
    assert_eq!(type_seq_vni("a3"), "ả");
    assert_eq!(type_seq_vni("a4"), "ã");
    assert_eq!(type_seq_vni("a5"), "ạ");
}

#[test]
fn vni_tone_removal() {
    // a1 -> á, then 0 -> a
    assert_eq!(type_seq_vni("a10"), "a");
    // a0 -> a
    assert_eq!(type_seq_vni("a0"), "a");
}

#[test]
fn vni_tones_on_modified_vowels() {
    // a6 + 1 => ấ
    assert_eq!(type_seq_vni("a61"), "ấ");
    // o6 + 1 => ố
    assert_eq!(type_seq_vni("o61"), "ố");
    // o7 + 1 => ớ
    assert_eq!(type_seq_vni("o71"), "ớ");
    // u7 + 1 => ứ
    assert_eq!(type_seq_vni("u71"), "ứ");
    // d9 + 1 should not tone (đ is not in mapping), stays đ
    assert_eq!(type_seq_vni("d91"), "đ");
}
