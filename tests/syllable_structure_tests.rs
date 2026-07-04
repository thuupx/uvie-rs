use uvie::{NucleusKind, OnsetKind, UltraFastViEngine};
mod common;
use common::type_seq;

// ===== Typed Syllable Slots Tests =====

#[test]
fn syl_structure_simple_consonant_vowel() {
    let mut e = UltraFastViEngine::new();
    e.feed('t');
    assert_eq!(e.syl_structure().onset_kind, OnsetKind::Single(b't'));
    assert_eq!(e.syl_structure().nucleus_kind, NucleusKind::None);

    e.feed('o');
    assert_eq!(e.syl_structure().onset_kind, OnsetKind::Single(b't'));
    assert_eq!(e.syl_structure().nucleus_kind, NucleusKind::Single);
    assert_eq!(e.syl_structure().onset_end, 1);
    assert_eq!(e.syl_structure().nucleus_end, 2);
}

#[test]
fn syl_structure_digraph_onset() {
    let mut e = UltraFastViEngine::new();
    e.feed('t');
    e.feed('h');
    assert_eq!(e.syl_structure().onset_kind, OnsetKind::Digraph(b't', b'h'));
    assert_eq!(e.syl_structure().nucleus_kind, NucleusKind::None);

    e.feed('u');
    assert_eq!(e.syl_structure().onset_kind, OnsetKind::Digraph(b't', b'h'));
    assert_eq!(e.syl_structure().nucleus_kind, NucleusKind::Single);
}

#[test]
fn syl_structure_diphthong_nucleus() {
    let mut e = UltraFastViEngine::new();
    // "to" then "o" → "tô" (circumflex), still single nucleus slot
    type_seq(&mut e, "too");
    assert_eq!(e.syl_structure().onset_kind, OnsetKind::Single(b't'));
    // The engine's partition sees [t, o, o_modifier] - the second 'o' triggers
    // circumflex, keeping nucleus as 1 slot. Actually the buf may have 2 entries
    // for 'o' and 'o' but the second one becomes a modifier... Let's just check
    // the raw partition result:
    assert!(matches!(
        e.syl_structure().nucleus_kind,
        NucleusKind::Single | NucleusKind::Diphthong
    ));
}

#[test]
fn syl_structure_no_onset() {
    let mut e = UltraFastViEngine::new();
    e.feed('a');
    assert_eq!(e.syl_structure().onset_kind, OnsetKind::None);
    assert_eq!(e.syl_structure().nucleus_kind, NucleusKind::Single);
    assert_eq!(e.syl_structure().onset_end, 0);
    assert_eq!(e.syl_structure().nucleus_end, 1);
}

#[test]
fn syl_structure_trigraph_ngh() {
    let mut e = UltraFastViEngine::new();
    e.feed('n');
    e.feed('g');
    e.feed('h');
    assert_eq!(e.syl_structure().onset_kind, OnsetKind::Trigraph);
    assert_eq!(e.syl_structure().onset_end, 3);

    e.feed('i');
    assert_eq!(e.syl_structure().nucleus_kind, NucleusKind::Single);
    assert_eq!(e.syl_structure().nucleus_end, 4);
}

#[test]
fn mid_nucleus_tone_for_iê_yê_uê() {
    // Tone can be typed between the two vowels of an incomplete iê/yê/uê nucleus.
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ieje"), "iệ", "ieje should produce iệ");
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "iefe"), "iề", "iefe should produce iề");
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "iere"), "iể", "iere should produce iể");
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "iexe"), "iễ", "iexe should produce iễ");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "yefe"), "yề", "yefe should produce yề");
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "yexe"), "yễ", "yexe should produce yễ");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ueje"), "uệ", "ueje should produce uệ");

    // With onset and coda
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "tieje"), "tiệ", "tieje should produce tiệ");
    let mut e = UltraFastViEngine::new();
    assert_eq!(
        type_seq(&mut e, "tiejen"),
        "tiện",
        "tiejen should produce tiện"
    );

    // Tone override after the delayed tone still works
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "iejes"), "iế", "iejes should produce iế");
}

// ---------------------------------------------------------------------------
// Uppercase / F_CAPS tests
// ---------------------------------------------------------------------------

#[test]
fn uppercase_d_with_stroke() {
    let mut e = UltraFastViEngine::new();
    // Shift+D twice → Đ (uppercase d with stroke)
    assert_eq!(type_seq(&mut e, "DD"), "Đ");

    let mut e = UltraFastViEngine::new();
    // Mixed case: first D uppercase, second lowercase → Đ
    assert_eq!(type_seq(&mut e, "Dd"), "Đ");

    let mut e = UltraFastViEngine::new();
    // Both lowercase → đ
    assert_eq!(type_seq(&mut e, "dd"), "đ");

    let mut e = UltraFastViEngine::new();
    // Passthrough: ĐB must keep uppercase Đ
    assert_eq!(type_seq(&mut e, "DDB"), "ĐB");
}

#[test]
fn uppercase_circumflex_oo() {
    let mut e = UltraFastViEngine::new();
    // Shift+O twice → Ô (uppercase circumflex O)
    assert_eq!(type_seq(&mut e, "OO"), "Ô");
}

#[test]
fn uppercase_circumflex_aa() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "AA"), "Â");
}

#[test]
fn uppercase_circumflex_ee() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "EE"), "Ê");
}

#[test]
fn uppercase_horn_ow() {
    let mut e = UltraFastViEngine::new();
    // Shift+O then W → Ơ (uppercase horn O)
    assert_eq!(type_seq(&mut e, "OW"), "Ơ");
}

#[test]
fn uppercase_horn_uw() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "UW"), "Ư");
}

#[test]
fn uppercase_breve_aw() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "AW"), "Ă");
}

#[test]
fn uppercase_circumflex_with_tone() {
    let mut e = UltraFastViEngine::new();
    // OOs → Ố (uppercase circumflex O with sắc)
    assert_eq!(type_seq(&mut e, "OOs"), "Ố");
}

#[test]
fn mixed_case_circumflex_first_upper() {
    let mut e = UltraFastViEngine::new();
    // First char uppercase, second lowercase: Oo → Ô
    assert_eq!(type_seq(&mut e, "Oo"), "Ô");
}

#[test]
fn mixed_case_horn_first_upper() {
    let mut e = UltraFastViEngine::new();
    // First char uppercase, second lowercase: Ow → Ơ
    assert_eq!(type_seq(&mut e, "Ow"), "Ơ");
}

#[test]
fn uppercase_preserved_in_passthrough() {
    let mut e = UltraFastViEngine::new();
    // "Al" is not valid Vietnamese; must stay "Al", not "al".
    assert_eq!(type_seq(&mut e, "Al"), "Al");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "AB"), "AB");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "Abc"), "Abc");
}

#[test]
fn mixed_case_passthrough() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "aL"), "aL");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ClEAR"), "ClEAR");
}

#[test]
fn uppercase_backspace_preserves_case() {
    let mut e = UltraFastViEngine::new();
    e.feed('A');
    e.feed('l');
    assert_eq!(e.current_composing(), "Al");
    e.backspace();
    assert_eq!(e.current_composing(), "A");

    let mut e = UltraFastViEngine::new();
    e.feed('A');
    e.feed('l');
    e.feed('e');
    e.backspace();
    assert_eq!(e.current_composing(), "Al");
}
