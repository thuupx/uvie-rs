use uvie::{InputMethod, UltraFastViEngine};
mod common;
use common::{type_seq, type_seq_vni};

#[test]
fn regression_user_reported_words() {
    // chuaw -> chưa (w bubbles back to u and turns it into ư)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "chuaw"),
        "chưa",
        "chuaw should produce chưa"
    );

    // chuyến / huyễn also need the standard Telex path to work
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "chuyenes"),
        "chuyến",
        "chuyenes should produce chuyến"
    );

    // chuýên -> chuyến (pre-accented y should be treated as base y)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "chuýên"),
        "chuyến",
        "chuýên should produce chuyến"
    );

    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "huyeenx"),
        "huyễn",
        "huyeenx should produce huyễn"
    );

    // huỹên -> huyễn (pre-accented y with ngã should be treated as base y)
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "huỹên"),
        "huyễn",
        "huỹên should produce huyễn"
    );

    // Same words in VNI mode with composed characters
    let mut e = UltraFastViEngine::new();
    e.set_input_method(InputMethod::Vni);
    assert_eq!(
        type_seq(&mut e, "chuýên"),
        "chuyến",
        "VNI: chuýên should produce chuyến"
    );

    let mut e = UltraFastViEngine::new();
    e.set_input_method(InputMethod::Vni);
    assert_eq!(
        type_seq(&mut e, "huỹên"),
        "huyễn",
        "VNI: huỹên should produce huyễn"
    );
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
    // az -> az (z is consonant when no tone to cancel)
    assert_eq!(type_seq(&mut e, "az"), "az");

    let mut e = UltraFastViEngine::new();
    // axz -> a (x sets ngã, z cancels)
    assert_eq!(type_seq(&mut e, "axz"), "a");
}

#[test]
fn toggling_triplet() {
    // New behaviour: triple cancel outputs the TWO literal chars before the
    // cancelling keystroke, not just one.  "nee"→"nê", "neee"→"nee" (literal).

    let mut e = UltraFastViEngine::new();
    // aaa → aa  (triple cancels "aa"→"â", keeps both a's literal)
    assert_eq!(type_seq(&mut e, "aaa"), "aa");

    let mut e = UltraFastViEngine::new();
    // ddd → dd  (triple cancels "dd"→"đ", keeps both d's literal)
    assert_eq!(type_seq(&mut e, "ddd"), "dd");

    let mut e = UltraFastViEngine::new();
    // eee → ee  (triple cancels "ee"→"ê", keeps both e's literal)
    assert_eq!(type_seq(&mut e, "eee"), "ee");

    let mut e = UltraFastViEngine::new();
    // ooo → oo  (triple cancels "oo"→"ô", keeps both o's literal)
    assert_eq!(type_seq(&mut e, "ooo"), "oo");

    // Pair still works normally (only 2 chars)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ee"), "ê");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "aa"), "â");
}

#[test]
fn triple_cancel_with_trailing_chars() {
    // Characters after a triple-cancel must be preserved, not silently dropped.
    // Bug: "neeeb" was outputting "nee" (losing 'b') because the early exit
    // only took bytes_all[..end] and discarded everything after the cancelling char.

    let mut e = UltraFastViEngine::new();
    // nee → nê, neee → nee (triple cancel), neeeb → neeb (b preserved)
    assert_eq!(type_seq(&mut e, "neeeb"), "neeb");

    let mut e = UltraFastViEngine::new();
    // neeeboo → neeboo (full raw passthrough after triple cancel)
    assert_eq!(type_seq(&mut e, "neeeboo"), "neeboo");

    let mut e = UltraFastViEngine::new();
    // aaaa → aaa? No: "aa" → "â", "aaa" → "aa" (cancel), "aaaa" → "aaa" (skip 3rd 'a')
    // Actually: aaa = ['a','a','a'], end=2, skip 'a' at 2 → "aa"
    // aaaa = ['a','a','a','a'], end=2, skip 'a' at 2 → ['a','a','a'] → "aaa"
    assert_eq!(type_seq(&mut e, "aaaa"), "aaa");

    let mut e = UltraFastViEngine::new();
    // With consonant prefix: "neeeee" → "neee"
    // neeee: ['n','e','e','e','e'], end=3 (3rd 'e'), skip 'e' at 3 → "neee"
    assert_eq!(type_seq(&mut e, "neeee"), "neee");
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
    e.set_modern_orthography(true);
    assert_eq!(type_seq(&mut e, "hoas"), "hoá");

    let mut e = UltraFastViEngine::new();
    e.set_modern_orthography(true);
    assert_eq!(type_seq(&mut e, "hoaf"), "hoà");
}

#[test]
fn tone_placement_two_vowels_with_coda() {
    let mut e = UltraFastViEngine::new();
    e.set_modern_orthography(true);
    assert_eq!(type_seq(&mut e, "hoans"), "hoán");

    let mut e = UltraFastViEngine::new();
    e.set_modern_orthography(true);
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
fn whitespace_commits_and_resets_composing() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "aas"), "ấ");
    // Space commits the composing word; feed returns empty composing text.
    assert_eq!(e.feed(' '), "");
    assert_eq!(e.committed_text(), "ấ ");
    assert_eq!(e.current_composing(), "");
    // New word starts with a fresh composing buffer.
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
    // oa -> hoà (tone on a, modern orthography)
    let mut e = UltraFastViEngine::new();
    e.set_modern_orthography(true);
    assert_eq!(type_seq(&mut e, "hoaf"), "hoà");

    // oe -> hoè (tone on e, modern orthography)
    let mut e = UltraFastViEngine::new();
    e.set_modern_orthography(true);
    assert_eq!(type_seq(&mut e, "hoef"), "hoè");

    // uy -> tuỳ (tone on y, modern orthography)
    let mut e = UltraFastViEngine::new();
    e.set_modern_orthography(true);
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
fn regression_ui_tone_on_first_vowel() {
    let mut e = UltraFastViEngine::new();
    // guiwr -> gửi (tone on ư, not on i)
    assert_eq!(type_seq(&mut e, "guiwr"), "gửi");
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
    // a0 -> a0 (0 is literal when no tone to cancel)
    assert_eq!(type_seq_vni("a0"), "a0");
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

#[test]
fn tone_on_modified_vowel_oi() {
    // mơí -> mới (tone on ơ, not i)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "mowis"), "mới");
}

#[test]
fn tone_on_modified_vowel_eu() {
    // nêú -> nếu (tone on ê, not u)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "neeus"), "nếu");
}

#[test]
fn double_tone_key_undoes_tone() {
    // tess -> test (double s undoes the tone, s becomes literal)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "tess"), "tes");

    // teff -> tef
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "teff"), "tef");

    // terr -> ter
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "terr"), "ter");

    // texx -> tex
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "texx"), "tex");

    // tejj -> tej
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "tejj"), "tej");
}

#[test]
fn double_w_undoes_modification() {
    // showw -> show (double w undoes ơ)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "showw"), "show");

    // oww -> ow
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "oww"), "ow");

    // uww -> uw
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "uww"), "uw");
}

#[test]
fn consonant_only_no_duplication() {
    // txt should stay txt (no duplication)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "txt"), "txt");

    // sx should stay sx
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "sx"), "sx");
}

#[test]
fn double_tone_then_continue() {
    // vieetj -> việt (double e makes ê, then tone j)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "vieetj"), "việt");
}

#[test]
fn tone_placement_oi_pair() {
    // đời -> ddowif
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddowif"), "đời");

    // tối -> toois
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "toois"), "tối");

    // lối -> loois
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "loois"), "lối");
}

#[test]
fn tone_placement_eu_pair() {
    // nếu -> neeus
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "neeus"), "nếu");

    // kều -> keeuf (tone f = huyền)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "keeuf"), "kều");

    // kểu -> keeur (tone r = hỏi)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "keeur"), "kểu");
}
