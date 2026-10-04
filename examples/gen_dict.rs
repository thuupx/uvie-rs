//! English override dictionary generator.
//!
//! Rule (per AGENTS.md design): include an English word iff its Telex
//! feed_diff render differs from the raw word AND the rendered text is NOT
//! valid Vietnamese — where "valid" means the whole rendered token can be
//! segmented into real Vietnamese syllables (from the 22k word list, tone
//! marks stripped). This covers the two documented exclusions:
//!   - transform is a real Vietnamese word        ("chaos"→"cháo")
//!   - V-C-V components both valid Vietnamese     ("user"→"u"+"sẻ")
use std::collections::{HashSet, HashMap};
use uvie::UltraFastViEngine;
use uvie::diff::Diffable;

// Override is disabled on the generator engine so we see the raw transform
// the dictionary is meant to fix (feed_diff applies the override per-keystroke).
fn type_word(engine: &mut UltraFastViEngine, input: &str) -> String {
    let mut screen = String::new();
    for c in input.chars() {
        let (bs, suffix) = engine.feed_diff(c);
        for _ in 0..bs { screen.pop(); }
        screen.push_str(suffix);
    }
    screen
}

/// Strip Vietnamese tone marks but keep vowel-quality marks (ă â ê ô ơ ư đ).
#[allow(dead_code)]
fn strip_tones(c: char) -> char {
    match c {
        'á'|'à'|'ả'|'ã'|'ạ' => 'a',
        'ấ'|'ầ'|'ẩ'|'ẫ'|'ậ' => 'â',
        'ắ'|'ằ'|'ẳ'|'ẵ'|'ặ' => 'ă',
        'é'|'è'|'ẻ'|'ẽ'|'ẹ' => 'e',
        'ế'|'ề'|'ể'|'ễ'|'ệ' => 'ê',
        'í'|'ì'|'ỉ'|'ĩ'|'ị' => 'i',
        'ó'|'ò'|'ỏ'|'õ'|'ọ' => 'o',
        'ố'|'ồ'|'ổ'|'ỗ'|'ộ' => 'ô',
        'ớ'|'ờ'|'ở'|'ỡ'|'ợ' => 'ơ',
        'ú'|'ù'|'ủ'|'ũ'|'ụ' => 'u',
        'ứ'|'ừ'|'ử'|'ữ'|'ự' => 'ư',
        'ý'|'ỳ'|'ỷ'|'ỹ'|'ỵ' => 'y',
        'Á'|'À'|'Ả'|'Ã'|'Ạ' => 'A',
        'Ấ'|'Ầ'|'Ẩ'|'Ẫ'|'Ậ' => 'Â',
        'Ắ'|'Ằ'|'Ẳ'|'Ẵ'|'Ặ' => 'Ă',
        'É'|'È'|'Ẻ'|'Ẽ'|'Ẹ' => 'E',
        'Ế'|'Ề'|'Ể'|'Ễ'|'Ệ' => 'Ê',
        'Í'|'Ì'|'Ỉ'|'Ĩ'|'Ị' => 'I',
        'Ó'|'Ò'|'Ỏ'|'Õ'|'Ọ' => 'O',
        'Ố'|'Ồ'|'Ổ'|'Ỗ'|'Ộ' => 'Ô',
        'Ớ'|'Ờ'|'Ở'|'Ỡ'|'Ợ' => 'Ơ',
        'Ú'|'Ù'|'Ủ'|'Ũ'|'Ụ' => 'U',
        'Ứ'|'Ừ'|'Ử'|'Ữ'|'Ự' => 'Ư',
        'Ý'|'Ỳ'|'Ỷ'|'Ỹ'|'Ỵ' => 'Y',
        c => c,
    }
}

/// Vietnamese onset consonant letters (first letter of every onset,
/// including digraphs ch gh gi kh ng ngh nh ph qu th tr).
fn is_vi_consonant(c: char) -> bool {
    matches!(c, 'b'|'c'|'d'|'đ'|'g'|'h'|'k'|'l'|'m'|'n'|'p'|'q'|'r'|'s'|'t'|'v'|'x')
}

fn main() {
    // Set of real Vietnamese WORDS (tones kept) — every whitespace-separated
    // token of the 22k list. The exclusion check is tone-aware: a rendered
    // token only counts as valid Vietnamese if it decomposes into real
    // words, not merely real syllable *shapes*.
    let vi_data = include_str!("/Users/devin/repos/uvie-rs/tests/data/vietnamese_22k.txt");
    let mut words: HashSet<String> = HashSet::new();
    for line in vi_data.lines() {
        for tok in line.split_whitespace() {
            let lw: String = tok.chars().map(|c| c.to_ascii_lowercase()).collect();
            if lw.chars().all(|c| c.is_alphabetic()) {
                words.insert(lw);
            }
        }
    }
    println!("word set: {}", words.len());

    // DP: can `s` (the rendered text, tones kept, lowercase) be decomposed
    // into real Vietnamese words where every cut lands on a consonant
    // onset — the same boundary the engine's V-C-V split produces? That is
    // the documented exclusion: the render itself is a real word, or its
    // split components are all real words ("u"+"sẻ").
    fn valid_vi(s: &str, words: &HashSet<String>, memo: &mut HashMap<String, bool>) -> bool {
        if s.is_empty() { return true; }
        if words.contains(s) { return true; }
        if let Some(&v) = memo.get(s) { return v; }
        let mut ok = false;
        for (i, _) in s.char_indices().skip(1) {
            let (pre, post) = s.split_at(i);
            let Some(first) = post.chars().next() else { continue };
            if words.contains(pre) && is_vi_consonant(first) && valid_vi(post, words, memo) {
                ok = true; break;
            }
        }
        memo.insert(s.to_string(), ok);
        ok
    }

    let en_data = include_str!("/Users/devin/repos/uvie-rs/tests/data/english_100k.txt");

    // Prefix-shadow drop: a dictionary word that is a prefix of a real
    // Telex keystroke sequence fires the per-keystroke override mid-word
    // and hijacks Vietnamese typing ("tojo" prefixes "tojot" → "tột").
    // Rare English loses to Vietnamese intent; common English (top ~20k
    // of the frequency-ordered corpus) wins — the same balance the
    // curated list struck ("dust", "data").
    let pairs_data = include_str!("/Users/devin/repos/uvie-rs/tests/data/vietnamese_telex_pairs.txt");
    let mut pair_inputs: Vec<&str> = pairs_data
        .lines()
        .filter_map(|l| l.split('\t').next())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    pair_inputs.sort();
    let top20k: HashSet<&str> = en_data
        .lines()
        .take(20_000)
        .map(|l| l.trim())
        .collect();
    let shadows_pair = |w: &str| -> bool {
        // First pair input >= w; if it starts with w, w is a prefix.
        let idx = pair_inputs.partition_point(|p| *p < w);
        idx < pair_inputs.len() && pair_inputs[idx].starts_with(w)
    };

    let mut dict: Vec<String> = Vec::new();
    let mut n_passthrough = 0;
    let mut n_valid_vi = 0;
    let mut n_shadowed = 0;
    for line in en_data.lines() {
        let word = line.trim();
        if word.len() < 4 || !word.chars().all(|c| c.is_ascii_lowercase()) { continue; }
        let mut e = UltraFastViEngine::new();
        e.set_modern_orthography(false);
        e.set_english_override(false);
        let rendered = type_word(&mut e, word);
        if rendered == word { n_passthrough += 1; continue; }
        let lw_render: String = rendered.chars().map(|c| c.to_ascii_lowercase()).collect();
        let mut memo = HashMap::new();
        if valid_vi(&lw_render, &words, &mut memo) { n_valid_vi += 1; continue; }
        if shadows_pair(word) && !top20k.contains(word) { n_shadowed += 1; continue; }
        dict.push(word.to_string());
    }
    dict.sort();
    dict.dedup();
    println!("passthrough: {}, valid-vi(excluded): {}, shadowed: {}, dict: {}", n_passthrough, n_valid_vi, n_shadowed, dict.len());
    use std::io::Write;
    let mut f = std::fs::File::create("/tmp/new_dict.txt").unwrap();
    for w in &dict { writeln!(f, "    \"{}\",", w).unwrap(); }
    // Show which current dict words are NOT in new list (regressions to check)
    let cur: HashSet<String> = include_str!("/Users/devin/repos/uvie-rs/src/tables/english.rs")
        .lines().filter_map(|l| l.trim().strip_prefix('"').and_then(|s| s.strip_suffix("\",\"").or_else(|| s.strip_suffix('"')))).map(|s| s.trim_end_matches(',').to_string()).collect();
    let new: HashSet<String> = dict.iter().cloned().collect();
    let missing: Vec<_> = cur.difference(&new).collect();
    println!("current-dict words absent from new dict: {}", missing.len());
    let mut f2 = std::fs::File::create("/tmp/missing_from_new.txt").unwrap();
    for w in &missing { writeln!(f2, "{}", w).unwrap(); }
}
