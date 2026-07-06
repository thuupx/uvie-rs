//! Unit tests for the `tables` module (onset/coda/nucleus legality, tone targets).

use uvie::tables::{
    is_legal_coda, is_legal_nucleus, is_legal_onset, is_onset_prefix, nucleus_tone_target,
    tone_allowed_for_coda,
};

#[test]
fn onset_single_chars() {
    assert!(is_legal_onset(b"b"));
    assert!(is_legal_onset(b"t"));
    assert!(is_legal_onset(b"n"));
    assert!(is_legal_onset(b""));
}

#[test]
fn onset_digraphs() {
    assert!(is_legal_onset(b"ph"));
    assert!(is_legal_onset(b"th"));
    assert!(is_legal_onset(b"tr"));
    assert!(is_legal_onset(b"gi"));
    assert!(is_legal_onset(b"ch"));
    assert!(is_legal_onset(b"nh"));
    assert!(is_legal_onset(b"ng"));
    assert!(is_legal_onset(b"kh"));
    assert!(is_legal_onset(b"gh"));
    assert!(is_legal_onset(b"qu"));
}

#[test]
fn onset_ngh() {
    assert!(is_legal_onset(b"ngh"));
}

#[test]
fn onset_illegal() {
    assert!(!is_legal_onset(b"tt")); // double consonant
    assert!(!is_legal_onset(b"cl")); // English cluster
    assert!(!is_legal_onset(b"str")); // 3-char non-ngh
    assert!(!is_legal_onset(b"bl"));
}

#[test]
fn coda_single() {
    assert!(is_legal_coda(b"t", false));
    assert!(is_legal_coda(b"n", false));
    assert!(is_legal_coda(b"m", false));
    assert!(is_legal_coda(b"c", false));
    assert!(is_legal_coda(b"p", false));
    assert!(is_legal_coda(b"i", false));
    assert!(is_legal_coda(b"y", false));
    assert!(is_legal_coda(b"u", false));
    assert!(is_legal_coda(b"", false));
    // relaxed mode: lone g/h as shorthand for ng/nh
    assert!(is_legal_coda(b"g", true));
    assert!(is_legal_coda(b"h", true));
    assert!(!is_legal_coda(b"g", false));
    assert!(!is_legal_coda(b"h", false));
    // teen-code: k as shorthand for c — always active (no toggle)
    assert!(is_legal_coda(b"k", false));
    assert!(is_legal_coda(b"k", true));
}

#[test]
fn coda_digraph() {
    assert!(is_legal_coda(b"ng", false));
    assert!(is_legal_coda(b"nh", false));
    assert!(is_legal_coda(b"ch", false));
    // teen-code: nk as shorthand for nh — always active (no toggle)
    assert!(is_legal_coda(b"nk", false));
    assert!(is_legal_coda(b"nk", true));
}

#[test]
fn coda_illegal() {
    assert!(!is_legal_coda(b"tt", false));
    assert!(!is_legal_coda(b"ll", false));
    assert!(!is_legal_coda(b"ngg", false));
}

#[test]
fn tone_coda_constraint() {
    // c/ch/p/t only allow sắc(1) and nặng(5)
    assert!(tone_allowed_for_coda(b"c", 1, false));
    assert!(tone_allowed_for_coda(b"c", 5, false));
    assert!(!tone_allowed_for_coda(b"c", 3, false));
    assert!(!tone_allowed_for_coda(b"c", 4, false));
    assert!(tone_allowed_for_coda(b"ch", 1, false));
    assert!(!tone_allowed_for_coda(b"ch", 3, false));
    // n/m/ng are free
    assert!(tone_allowed_for_coda(b"n", 3, false));
    assert!(tone_allowed_for_coda(b"ng", 4, false));
    assert!(tone_allowed_for_coda(b"", 3, false));
    // relaxed mode: lone g/h behave like ng/nh (allow any tone)
    assert!(tone_allowed_for_coda(b"g", 3, true));
    assert!(tone_allowed_for_coda(b"g", 4, true));
    assert!(tone_allowed_for_coda(b"h", 3, true));
    assert!(tone_allowed_for_coda(b"h", 2, true));
    // teen-code: k behaves like c (stopped coda — sắc/nặng only), always active
    assert!(tone_allowed_for_coda(b"k", 1, false)); // sắc
    assert!(tone_allowed_for_coda(b"k", 5, false)); // nặng
    assert!(!tone_allowed_for_coda(b"k", 2, false)); // huyền
    assert!(!tone_allowed_for_coda(b"k", 3, false)); // hỏi
    assert!(!tone_allowed_for_coda(b"k", 4, true)); // ngã — even in relaxed
    // teen-code: nk behaves like nh (any tone), always active
    assert!(tone_allowed_for_coda(b"nk", 1, false)); // sắc
    assert!(tone_allowed_for_coda(b"nk", 2, false)); // huyền
    assert!(tone_allowed_for_coda(b"nk", 3, false)); // hỏi
    assert!(tone_allowed_for_coda(b"nk", 4, false)); // ngã
    assert!(tone_allowed_for_coda(b"nk", 5, false)); // nặng
}

#[test]
fn nucleus_single_vowels() {
    for &v in &['a', 'ă', 'â', 'e', 'ê', 'i', 'o', 'ô', 'ơ', 'u', 'ư', 'y'] {
        assert_eq!(
            nucleus_tone_target(&[v]),
            Some(0),
            "vowel {:?} should be legal",
            v
        );
    }
}

#[test]
fn nucleus_diphthongs_tone_target() {
    // oa → tone on a (index 1): "hoá" = hoas
    assert_eq!(nucleus_tone_target(&['o', 'a']), Some(1));
    // oe → tone on e (index 1): "hoè"
    assert_eq!(nucleus_tone_target(&['o', 'e']), Some(1));
    // ưi → tone on ư (index 0): "gửi"
    assert_eq!(nucleus_tone_target(&['ư', 'i']), Some(0));
    // êu → tone on ê (index 0): "nếu"
    assert_eq!(nucleus_tone_target(&['ê', 'u']), Some(0));
    // iê → tone on ê (index 1): "tiến"
    assert_eq!(nucleus_tone_target(&['i', 'ê']), Some(1));
    // uy → tone on y (index 1): "tuỳ" (modern orthography)
    assert_eq!(nucleus_tone_target(&['u', 'y']), Some(1));
}

#[test]
fn nucleus_triphthong_uyê() {
    // uyê → tone on ê (index 2): "quyết"
    assert_eq!(nucleus_tone_target(&['u', 'y', 'ê']), Some(2));
}

#[test]
fn nucleus_triphthong_oai() {
    assert_eq!(nucleus_tone_target(&['o', 'a', 'i']), Some(1));
}

#[test]
fn nucleus_illegal() {
    assert_eq!(nucleus_tone_target(&['e', 'l']), None);
    assert_eq!(nucleus_tone_target(&['a', 'l']), None);
    assert_eq!(nucleus_tone_target(&[]), None);
}

#[test]
fn onset_prefix_valid() {
    assert!(is_onset_prefix(b"n"));
    assert!(is_onset_prefix(b"ng"));
    assert!(is_onset_prefix(b"ngh"));
    assert!(is_onset_prefix(b"p"));
    assert!(is_onset_prefix(b"ph"));
}

#[test]
fn onset_prefix_invalid() {
    assert!(!is_onset_prefix(b"tt"));
    assert!(!is_onset_prefix(b"bl"));
}
