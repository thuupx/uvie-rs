use uvie::UltraFastViEngine;
use uvie::diff::Diffable;
mod common;
use common::type_seq;

// ===== Comprehensive edge case tests =====

#[test]
fn edge_double_tone_various_positions() {
    // Double tone at end of word with vowel
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "bass"), "bas");

    // Double tone in middle then more chars - cancelled tone key becomes literal, extra chars accepted
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "tesstt"), "testt");

    // zz: first z is consonant (no tone to cancel), second z is also consonant
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "azz"), "azz");
}

#[test]
fn edge_double_w_various() {
    // aww -> aw (undo ă)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "aww"), "aw");

    // ddoww -> đow (undo ơ, keep đ)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddoww"), "đow");
}

#[test]
fn edge_english_words_passthrough() {
    // Common English words that contain tone keys
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "stress"), "stress");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "jazz"), "jazz");

    // Pure consonant sequences
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "txt"), "txt");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "rx"), "rx");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "sx"), "sx");
}

#[test]
fn edge_modified_vowel_tone_placement() {
    // ươi -> tone on ơ (second in ươ pair)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "huowis"), "hưới");

    // ươn -> tone on ơ
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "huowns"), "hướn");

    // ươ alone -> tone on ơ
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "huows"), "hướ");

    // âu -> tone on â
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "daauf"), "dầu");

    // ây -> tone on â
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "daays"), "dấy");
}

#[test]
fn edge_consecutive_words_via_space() {
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "vieetj"), "việt");
    e.feed(' ');
    assert_eq!(e.current_composing(), "");
    assert_eq!(e.committed_text(), "việt ");
    assert_eq!(type_seq(&mut e, "namm"), "namm");
}

#[test]
fn edge_single_char_tone_keys() {
    // Single tone key chars should pass through as-is
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "s"), "s");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "f"), "f");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "r"), "r");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "x"), "x");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "j"), "j");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "z"), "z");
}

#[test]
fn edge_common_vietnamese_words() {
    // Common words that exercise multiple features
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "xins"), "xín");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "chaof"), "chào");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddeepj"), "đệp");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "nawm"), "năm");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "nawms"), "nắm");

    // không -> khoongf (ô + huyền = ồ)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "khoongf"), "khồng");

    // được -> dduowcj
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "dduowcj"), "được");

    // người -> nguowif
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "nguowif"), "người");
}

#[test]
fn free_style_modifier_bubbling() {
    // ee modifier with vowel in between: neues -> nếu
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "neues"), "nếu");

    // aa modifier with vowel in between: naos -> nâó? No - naos: n,a,o -> nao + tone s
    // Actually: nao with free-style aa: naoas -> n,a,o,a -> bubble a next to a -> n,a,a,o -> nâo + s -> nấo
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "naoas"), "nấo");

    // oo modifier with vowel in between: noies -> nối
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "noios"), "nối");

    // oo modifier bubbling past tone key: noiso -> nối
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "noiso"), "nối");

    // Free-style ee: tieengs -> tiếng
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "tieengs"), "tiếng");

    // Free-style with w: moiws -> mới
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "moiws"), "mới");

    // dd modifier across consonants: bubbles to đan (valid Vietnamese)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "dand"), "đan");

    // oo modifier bubbling past tone key: loixo -> lỗi
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "loixo"), "lỗi");
}

#[test]
fn relaxed_coda_allows_g_shorthand() {
    // Strict mode: lone g is not a legal coda -> passthrough
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddawjg"), "đawjg");

    // Relaxed mode: g is accepted as a legal coda and rendered verbatim
    // (the user types g as shorthand for ng; the engine keeps the typed char).
    let mut e = UltraFastViEngine::new();
    e.set_relaxed_coda(true);
    assert_eq!(type_seq(&mut e, "ddawjg"), "đặg");

    // Relaxed mode also works with other tones
    let mut e = UltraFastViEngine::new();
    e.set_relaxed_coda(true);
    assert_eq!(type_seq(&mut e, "ddasg"), "đág");

    // Standard ng still works in relaxed mode
    let mut e = UltraFastViEngine::new();
    e.set_relaxed_coda(true);
    assert_eq!(type_seq(&mut e, "ddawngj"), "đặng");
}

#[test]
fn relaxed_coda_allows_h_shorthand() {
    // Strict mode: lone h is not a legal coda -> passthrough
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "nhafh"), "nhafh");

    // Relaxed mode: h is accepted as a legal coda and rendered verbatim
    // (the user types h as shorthand for nh; the engine keeps the typed char).
    let mut e = UltraFastViEngine::new();
    e.set_relaxed_coda(true);
    assert_eq!(type_seq(&mut e, "nhafh"), "nhàh");

    // With nặng tone
    let mut e = UltraFastViEngine::new();
    e.set_relaxed_coda(true);
    assert_eq!(type_seq(&mut e, "ddawjh"), "đặh");

    // Standard nh still works in relaxed mode
    let mut e = UltraFastViEngine::new();
    e.set_relaxed_coda(true);
    assert_eq!(type_seq(&mut e, "ddanhj"), "đạnh");
}

#[test]
fn teen_coda_k_always_active_for_c() {
    // `k` is a shorthand for the stopped coda `c` and is ALWAYS active (no
    // toggle needed). It follows the same tone constraint as `c`: only sắc
    // and nặng tones are allowed. Rendered verbatim — the engine keeps the
    // typed `k` (e.g. "Đắk Lắk" province spelling, "đắk" teen code).

    // Strict mode (relaxed coda OFF): k is still accepted as a coda.
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddawsk"), "đắk"); // sắc tone OK
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddawjk"), "đặk"); // nặng tone OK

    // Non-stopped tones fall through to literal passthrough (like `c`).
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddawfk"), "đawfk"); // huyền -> passthrough
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddawrk"), "đawrk"); // hỏi -> passthrough

    // Province name "Đắk Lắk" — both syllables use the k coda.
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "Ddawk Lak"), "Đăk Lak");

    // Standard `c` coda still works alongside the k shorthand.
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddawsc"), "đắc");

    // Relaxed mode does not change k behaviour (already always active).
    let mut e = UltraFastViEngine::new();
    e.set_relaxed_coda(true);
    assert_eq!(type_seq(&mut e, "ddawsk"), "đắk");
}

#[test]
fn teen_coda_nk_always_active_for_nh() {
    // `nk` is a shorthand for `nh` and is ALWAYS active (no toggle needed).
    // It follows the same tone constraint as `nh`: any tone is allowed.
    // Rendered verbatim — the engine keeps the typed `nk` (e.g. "đỉnk" is
    // teen code for "đỉnh").

    // Strict mode (relaxed coda OFF): nk is still accepted as a coda and
    // allows all tones (like nh).
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddirnk"), "đỉnk"); // hỏi
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddifnk"), "đìnk"); // huyền
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddixnk"), "đĩnk"); // ngã
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddasnk"), "đánk"); // sắc
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddajnk"), "đạnk"); // nặng

    // Standard `nh` coda still works alongside the nk shorthand.
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ddirnh"), "đỉnh");

    // Relaxed mode does not change nk behaviour (already always active).
    let mut e = UltraFastViEngine::new();
    e.set_relaxed_coda(true);
    assert_eq!(type_seq(&mut e, "ddirnk"), "đỉnk");
}

#[test]
fn no_bubble_across_consonants() {
    // reset: e + s + e where 's' is a Telex tone key (sắc) → forms ê.
    // This is correct Vietnamese IME behavior: "reset" → "rết" because
    // 's' between two e's is interpreted as a tone key, not a consonant.
    // English passthrough is inherently limited for Vietnamese IMEs.
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "reset"), "rết");

    // electronic: e + l + e where 'l' is NOT a tone key → no bubble, stays "electronic"
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "electronic"), "electronic");

    // depend: e + p + e where 'p' is NOT a tone key → no bubble
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "depend"), "depend");

    // added: dd → đ by resolver, but "ađed" has V-C-V pattern → not a valid Vietnamese
    // syllable → engine falls back to raw passthrough "added"
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "added"), "added");

    // banana: a..a bubbles to â, but "bânna" has V-C-V pattern → raw passthrough
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "banana"), "banana");

    // resset: double-s cancels tone, but then e+s+e forms circumflex ê
    // with the sắc tone from the second s → "rết" (valid Vietnamese syllable).
    // This is expected behavior: 's' is a Telex tone key, so e+s+e → ế.
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "resset"), "rết");

    // Free-style still works when only vowels/w separate: neues -> nếu
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "neues"), "nếu");

    // Free-style across consonants with tone key: memef -> mềm
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "memef"), "mềm");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "nuotos"), "nuốt");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "thajta"), "thật");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "chuanar"), "chuẩn");

    // wwork: 'w' alone → ư nucleus, second 'w' double-cancel reverts ư → w literal,
    // subsequent ork continues as passthrough → "work" (like "dd"→"đ", "ddd"→"dd")
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "wwork"), "work");

    // Free-style across consonants without tone: nene -> nên
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "nene"), "nên");
}

#[test]
fn free_style_does_not_break_normal() {
    // Normal adjacent modifiers still work
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "aas"), "ấ");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ees"), "ế");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "oos"), "ố");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "dd"), "đ");

    // Triple cancel now outputs two literal chars (not one)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "aaa"), "aa");

    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "eee"), "ee");
}

#[test]
fn invalid_onset_pair_fallback() {
    // tl is not a valid Vietnamese onset -> fallback to raw
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "tl"), "tl");

    // bh is not valid -> fallback
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "bh"), "bh");

    // lr is not valid -> fallback
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "lr"), "lr");
}

#[test]
fn valid_onset_pairs() {
    // tr is valid
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "tras"), "trá");

    // ph is valid
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "phas"), "phá");

    // kh is valid
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "khas"), "khá");

    // ngh is valid
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "nghes"), "nghé");
}

#[test]
fn tone_restriction_ch_t_coda() {
    // ch + sac (1) is valid
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "achs"), "ách");

    // ch + hoi (3) is invalid -> fallback raw
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "achr"), "achr");

    // ch + nga (4) is invalid -> fallback raw
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "achx"), "achx");

    // t + sac is valid
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "ats"), "át");

    // t + hoi is invalid -> fallback raw
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "atr"), "atr");
}

#[test]
fn quick_start_consonants() {
    let mut e = UltraFastViEngine::new();
    e.set_quick_start(true);
    assert_eq!(type_seq(&mut e, "jang"), "giang");

    let mut e = UltraFastViEngine::new();
    e.set_quick_start(true);
    assert_eq!(type_seq(&mut e, "phanhs"), "phánh");

    let mut e = UltraFastViEngine::new();
    e.set_quick_start(true);
    assert_eq!(type_seq(&mut e, "wen"), "quen");
}

#[test]
fn quick_start_disabled_by_default() {
    let mut e = UltraFastViEngine::new();
    // j should remain literal when quick_start is off
    assert_eq!(type_seq(&mut e, "jang"), "jang");
}

#[test]
fn quick_telex_cc() {
    let mut e = UltraFastViEngine::new();
    e.set_quick_telex(true);
    assert_eq!(type_seq(&mut e, "cc"), "ch");
}

#[test]
fn quick_telex_gg() {
    let mut e = UltraFastViEngine::new();
    e.set_quick_telex(true);
    assert_eq!(type_seq(&mut e, "gg"), "gi");
}

#[test]
fn quick_telex_kk() {
    let mut e = UltraFastViEngine::new();
    e.set_quick_telex(true);
    assert_eq!(type_seq(&mut e, "kk"), "kh");
}

#[test]
fn quick_telex_nn() {
    let mut e = UltraFastViEngine::new();
    e.set_quick_telex(true);
    assert_eq!(type_seq(&mut e, "nn"), "ng");
}

#[test]
fn quick_telex_qq() {
    let mut e = UltraFastViEngine::new();
    e.set_quick_telex(true);
    assert_eq!(type_seq(&mut e, "qq"), "qu");
}

#[test]
fn quick_telex_pp() {
    let mut e = UltraFastViEngine::new();
    e.set_quick_telex(true);
    assert_eq!(type_seq(&mut e, "pp"), "ph");
}

#[test]
fn quick_telex_tt() {
    let mut e = UltraFastViEngine::new();
    e.set_quick_telex(true);
    assert_eq!(type_seq(&mut e, "tt"), "th");
}

#[test]
fn quick_telex_hh() {
    let mut e = UltraFastViEngine::new();
    e.set_quick_telex(true);
    assert_eq!(type_seq(&mut e, "hh"), "nh");
}

#[test]
fn quick_telex_with_tone() {
    let mut e = UltraFastViEngine::new();
    e.set_quick_telex(true);
    // ccas -> ch + a + s (tone sac) -> chá
    assert_eq!(type_seq(&mut e, "ccas"), "chá");
}

#[test]
fn quick_telex_disabled_by_default() {
    let mut e = UltraFastViEngine::new();
    // cc should stay cc when quick_telex is off
    assert_eq!(type_seq(&mut e, "cc"), "cc");
}

#[test]
fn modern_orthography_hoas() {
    // hoas -> hoá (tone on 'a' — modern orthography)
    let mut e = UltraFastViEngine::new();
    e.set_modern_orthography(true);
    assert_eq!(type_seq(&mut e, "hoas"), "hoá");
}

#[test]
fn traditional_orthography_hoas() {
    // hoas -> hóa (tone on 'o' — traditional orthography, engine default)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "hoas"), "hóa");
}

#[test]
fn modern_orthography_thuys() {
    // thuys -> thuý (tone on 'y' — modern orthography)
    let mut e = UltraFastViEngine::new();
    e.set_modern_orthography(true);
    assert_eq!(type_seq(&mut e, "thuys"), "thuý");
}

#[test]
fn traditional_orthography_thuys() {
    // thuys -> thúy (tone on 'u' — traditional orthography)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "thuys"), "thúy");
}

#[test]
fn modern_orthography_oa_with_coda() {
    // hoacs -> hoác (tone on 'a' even with coda — modern)
    let mut e = UltraFastViEngine::new();
    e.set_modern_orthography(true);
    assert_eq!(type_seq(&mut e, "hoacs"), "hoác");
}

#[test]
fn traditional_orthography_oa_with_coda() {
    // hoacs -> hoác — with any coda, tone goes on second vowel in both modes.
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "hoacs"), "hoác");
}

#[test]
fn traditional_orthography_oa_coda_all() {
    // All codas (stopped + nasal + glide) place tone on 'a' (second vowel).
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "hoajt"), "hoạt");
    let mut e2 = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e2, "hoast"), "hoát");
    let mut e3 = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e3, "hoapj"), "hoạp");
    let mut e4 = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e4, "hoachj"), "hoạch");
    // Nasal codas: n, m, ng, nh
    let mut e5 = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e5, "ddoans"), "đoán");
    let mut e6 = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e6, "hoanj"), "hoạn");
    let mut e7 = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e7, "hoamj"), "hoạm");
    let mut e8 = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e8, "hoangj"), "hoạng");
    let mut e9 = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e9, "hoanhj"), "hoạnh");
}

#[test]
fn traditional_orthography_oe_coda() {
    // hoét, hoèn — oe diphthong with coda, tone on 'e' (second vowel).
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "hoest"), "hoét");
    let mut e2 = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e2, "hoenf"), "hoèn");
}

#[test]
fn traditional_orthography_open_syllable_still_first_vowel() {
    // Open syllables (no coda) keep traditional first-vowel placement.
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "hoas"), "hóa");
    let mut e2 = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e2, "khoes"), "khóe");
    let mut e3 = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e3, "thuys"), "thúy");
}

#[test]
fn modern_orthography_oe_pair() {
    // khoes -> khoé (tone on 'e' — modern)
    let mut e = UltraFastViEngine::new();
    e.set_modern_orthography(true);
    assert_eq!(type_seq(&mut e, "khoes"), "khoé");
}

#[test]
fn traditional_orthography_oe_pair() {
    // khoes -> khóe (tone on 'o' — traditional)
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "khoes"), "khóe");
}

#[test]
fn modern_orthography_quy_prefix() {
    // qu + uy -> quý (qu prefix, tone on 'y')
    let mut e = UltraFastViEngine::new();
    assert_eq!(type_seq(&mut e, "quys"), "quý");
}

#[test]
fn quick_telex_english_words_passthrough() {
    let mut e = UltraFastViEngine::new();
    e.set_quick_telex(true);
    // "account" has 'cc' which gets expanded to 'ch' when quick telex is on
    assert_eq!(type_seq(&mut e, "account"), "achount");
}

#[test]
fn diff_compact_no_crash() {
    // Safety valve must prevent raw_chars from overflowing (capacity = 24).
    let mut e = UltraFastViEngine::new();

    // Feed 'n' then 40 'e' keys - without safety-valve this would crash.
    e.feed_diff('n');
    for _ in 0..40 {
        e.feed_diff('e');
    }
    // Should not crash. The exact output depends on safety-valve resets,
    // but it must be non-empty.
    let out = e.current_composing_diff();
    assert!(!out.is_empty(), "output should not be empty after 41 e's");
}

#[test]
fn diff_triple_cancel_preserves_trailing_chars() {
    // After triple-cancel, subsequent characters must be preserved, not silently dropped.
    let mut e = UltraFastViEngine::new();

    // nee → nê
    e.feed_diff('n');
    e.feed_diff('e');
    e.feed_diff('e');
    assert_eq!(e.current_composing_diff(), "nê");

    // neee → nee (triple cancel, 3rd 'e' skipped)
    e.feed_diff('e');
    assert_eq!(e.current_composing_diff(), "nee");

    // neeeb → neeb ('b' preserved after cancel)
    e.feed_diff('b');
    assert_eq!(e.current_composing_diff(), "neeb");

    // neeebo → neebo ('o' preserved)
    e.feed_diff('o');
    assert_eq!(e.current_composing_diff(), "neebo");

    // neeeboo → neeboo (full word preserved)
    e.feed_diff('o');
    assert_eq!(e.current_composing_diff(), "neeboo");
}
