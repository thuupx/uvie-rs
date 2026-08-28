//! Data-driven tests using word lists from a reference Vietnamese engine.
//!
//! Test data files in `tests/data/`:
//! - `vietnamese_telex_pairs.txt` — 30k input→output mappings for Telex
//! - `vietnamese_22k.txt` — 22k Vietnamese words for round-trip testing
//! - `english_100k.txt` — 100k English words for passthrough testing

mod common;

use uvie::{InputMethod, UltraFastViEngine};

/// Type a telex input string (with trailing space to commit) and return the
/// result. Uses the `feed()` API (same as `type_seq`).
fn type_telex(engine: &mut UltraFastViEngine, input: &str) -> String {
    let mut result = String::new();
    for c in input.chars() {
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

/// Parse a tab-separated `input<TAB>expected` line.
fn parse_pair(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut parts = line.splitn(2, '\t');
    let input = parts.next()?.trim();
    let expected = parts.next()?.trim();
    if input.is_empty() || expected.is_empty() {
        return None;
    }
    Some((input, expected))
}

#[test]
fn telex_pairs_coverage() {
    let data = include_str!("data/vietnamese_telex_pairs.txt");
    let mut passed = 0;
    let mut failed = 0;
    let mut failures: Vec<(&str, &str, String)> = Vec::new();

    for line in data.lines() {
        let Some((input, expected)) = parse_pair(line) else {
            continue;
        };

        let mut e = UltraFastViEngine::new();
        // Traditional orthography (default), matching the reference engine.
        e.set_modern_orthography(false);
        let typed = format!("{} ", input);
        let result = type_telex(&mut e, &typed);
        let actual = result.trim();

        if actual == expected {
            passed += 1;
        } else {
            failed += 1;
            if failures.len() < 2000 {
                failures.push((input, expected, actual.to_string()));
            }
        }
    }

    let total = passed + failed;
    let pass_rate = if total > 0 {
        (passed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    eprintln!("\n=== Telex Pairs Test Results ===");
    eprintln!("Total: {}", total);
    eprintln!("Passed: {} ({:.2}%)", passed, pass_rate);
    eprintln!("Failed: {}", failed);

    if !failures.is_empty() {
        eprintln!("\n=== First {} Failures ===", failures.len().min(50));
        eprintln!("{:<15} {:<15} {:<15}", "INPUT", "EXPECTED", "ACTUAL");
        for (input, expected, actual) in failures.iter().take(50) {
            eprintln!("{:<15} {:<15} {:<15}", input, expected, actual);
        }
        // Write all failures to a file for analysis
        if let Ok(mut f) = std::fs::File::create("tests/data/telex_pairs_failures.txt") {
            use std::io::Write;
            for (input, expected, actual) in &failures {
                let _ = writeln!(f, "{}\t{}\t{}", input, expected, actual);
            }
        }
    }

    // CI threshold: fail if pass rate drops below 99%.
    // Current rate: 99.98% (6 failures out of 30337, all rare edge cases
    // like "huow" → "huơ" vs actual "hươ").
    const MIN_PASS_RATE: f64 = 99.0;
    assert!(
        pass_rate >= MIN_PASS_RATE,
        "Telex pairs pass rate {:.2}% is below threshold {:.1}%",
        pass_rate,
        MIN_PASS_RATE
    );
}

/// Decompose a pre-composed Vietnamese character into its Telex keystroke
/// representation. Returns `(base_keys, tone_key)` where `tone_key` is `""`
/// when the character carries no tone mark. Returns `None` for characters
/// that pass through as-is (ASCII consonants, digits, etc.).
fn decompose_vietnamese(c: char) -> Option<(&'static str, &'static str)> {
    Some(match c {
        // Base modified letters (no tone)
        'â' => ("aa", ""),
        'ă' => ("aw", ""),
        'ê' => ("ee", ""),
        'ô' => ("oo", ""),
        'ơ' => ("ow", ""),
        'ư' => ("uw", ""),
        'đ' => ("dd", ""),
        'Đ' => ("dd", ""),

        // a + tone
        'á' => ("a", "s"),
        'à' => ("a", "f"),
        'ả' => ("a", "r"),
        'ã' => ("a", "x"),
        'ạ' => ("a", "j"),

        // â + tone
        'ấ' => ("aa", "s"),
        'ầ' => ("aa", "f"),
        'ẩ' => ("aa", "r"),
        'ẫ' => ("aa", "x"),
        'ậ' => ("aa", "j"),

        // ă + tone
        'ắ' => ("aw", "s"),
        'ằ' => ("aw", "f"),
        'ẳ' => ("aw", "r"),
        'ẵ' => ("aw", "x"),
        'ặ' => ("aw", "j"),

        // e + tone
        'é' => ("e", "s"),
        'è' => ("e", "f"),
        'ẻ' => ("e", "r"),
        'ẽ' => ("e", "x"),
        'ẹ' => ("e", "j"),

        // ê + tone
        'ế' => ("ee", "s"),
        'ề' => ("ee", "f"),
        'ể' => ("ee", "r"),
        'ễ' => ("ee", "x"),
        'ệ' => ("ee", "j"),

        // i + tone
        'í' => ("i", "s"),
        'ì' => ("i", "f"),
        'ỉ' => ("i", "r"),
        'ĩ' => ("i", "x"),
        'ị' => ("i", "j"),

        // o + tone
        'ó' => ("o", "s"),
        'ò' => ("o", "f"),
        'ỏ' => ("o", "r"),
        'õ' => ("o", "x"),
        'ọ' => ("o", "j"),

        // ô + tone
        'ố' => ("oo", "s"),
        'ồ' => ("oo", "f"),
        'ổ' => ("oo", "r"),
        'ỗ' => ("oo", "x"),
        'ộ' => ("oo", "j"),

        // ơ + tone
        'ớ' => ("ow", "s"),
        'ờ' => ("ow", "f"),
        'ở' => ("ow", "r"),
        'ỡ' => ("ow", "x"),
        'ợ' => ("ow", "j"),

        // u + tone
        'ú' => ("u", "s"),
        'ù' => ("u", "f"),
        'ủ' => ("u", "r"),
        'ũ' => ("u", "x"),
        'ụ' => ("u", "j"),

        // ư + tone
        'ứ' => ("uw", "s"),
        'ừ' => ("uw", "f"),
        'ử' => ("uw", "r"),
        'ữ' => ("uw", "x"),
        'ự' => ("uw", "j"),

        // y + tone
        'ý' => ("y", "s"),
        'ỳ' => ("y", "f"),
        'ỷ' => ("y", "r"),
        'ỹ' => ("y", "x"),
        'ỵ' => ("y", "j"),

        _ => return None,
    })
}

/// Convert a Vietnamese word or phrase into Telex keystrokes.
///
/// Each syllable is converted independently: every pre-composed Vietnamese
/// character is decomposed into its base keystrokes, and the tone key (if
/// any) is appended at the end of the syllable. Uppercase is preserved by
/// capitalizing the first letter of the decomposed base.
fn vietnamese_to_telex(text: &str) -> String {
    let mut result = String::new();
    for (i, syllable) in text.split_whitespace().enumerate() {
        if i > 0 {
            result.push(' ');
        }
        let mut tone_key = "";
        for c in syllable.chars() {
            match decompose_vietnamese(c) {
                Some((base, tone)) => {
                    if c.is_uppercase() {
                        // Capitalize the first letter of the base keystrokes.
                        let mut chars = base.chars();
                        if let Some(first) = chars.next() {
                            for u in first.to_uppercase() {
                                result.push(u);
                            }
                            result.push_str(chars.as_str());
                        }
                    } else {
                        result.push_str(base);
                    }
                    if !tone.is_empty() {
                        tone_key = tone;
                    }
                }
                None => result.push(c),
            }
        }
        result.push_str(tone_key);
    }
    result
}

#[test]
fn vietnamese_22k_round_trip() {
    let data = include_str!("data/vietnamese_22k.txt");
    let mut passed = 0;
    let mut failed = 0;
    let mut failures: Vec<(&str, String, String)> = Vec::new();

    for line in data.lines() {
        let word = line.trim();
        if word.is_empty() || word.starts_with('#') {
            continue;
        }

        let telex = vietnamese_to_telex(word);

        let mut e = UltraFastViEngine::new();
        e.set_modern_orthography(false);
        let typed = format!("{} ", telex);
        let result = type_telex(&mut e, &typed);
        let actual = result.trim();

        if actual == word {
            passed += 1;
        } else {
            failed += 1;
            if failures.len() < 2000 {
                failures.push((word, telex, actual.to_string()));
            }
        }
    }

    let total = passed + failed;
    let pass_rate = if total > 0 {
        (passed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    eprintln!("\n=== Vietnamese 22k Round-Trip Test Results ===");
    eprintln!("Total: {}", total);
    eprintln!("Passed: {} ({:.2}%)", passed, pass_rate);
    eprintln!("Failed: {}", failed);

    if !failures.is_empty() {
        eprintln!("\n=== First {} Failures ===", failures.len().min(50));
        eprintln!("{:<20} {:<20} {:<20}", "EXPECTED", "TELEX", "ACTUAL");
        for (word, telex, actual) in failures.iter().take(50) {
            eprintln!("{:<20} {:<20} {:<20}", word, telex, actual);
        }
        if let Ok(mut f) = std::fs::File::create("tests/data/round_trip_failures.txt") {
            use std::io::Write;
            for (word, telex, actual) in &failures {
                let _ = writeln!(f, "{}\t{}\t{}", word, telex, actual);
            }
        }
    }

    // CI threshold: round-trip should reconstruct the vast majority of words.
    // Failures are typically due to ambiguous Telex encodings or rare edge
    // cases in tone placement orthography.
    const MIN_PASS_RATE: f64 = 95.0;
    assert!(
        pass_rate >= MIN_PASS_RATE,
        "Round-trip pass rate {:.2}% is below threshold {:.1}%",
        pass_rate,
        MIN_PASS_RATE
    );
}

#[test]
fn english_100k_passthrough() {
    let data = include_str!("data/english_100k.txt");
    let mut passed = 0;
    let mut failed = 0;
    let mut failures: Vec<(&str, String)> = Vec::new();

    for line in data.lines() {
        let word = line.trim();
        if word.is_empty() {
            continue;
        }

        // Skip words that contain non-ASCII (shouldn't happen in English list)
        if !word.is_ascii() {
            continue;
        }

        let mut e = UltraFastViEngine::new();
        e.set_modern_orthography(false);
        let typed = format!("{} ", word);
        let result = type_telex(&mut e, &typed);
        let actual = result.trim();

        // English words should pass through unchanged
        if actual.eq_ignore_ascii_case(word) {
            passed += 1;
        } else {
            failed += 1;
            if failures.len() < 200 {
                failures.push((word, actual.to_string()));
            }
        }
    }

    let total = passed + failed;
    let pass_rate = if total > 0 {
        (passed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    eprintln!("\n=== English 100k Passthrough Test Results ===");
    eprintln!("Total: {}", total);
    eprintln!("Passed: {} ({:.2}%)", passed, pass_rate);
    eprintln!("Failed: {}", failed);

    if !failures.is_empty() {
        eprintln!("\n=== First {} Failures ===", failures.len().min(50));
        eprintln!("{:<20} {:<20}", "ENGLISH", "ACTUAL");
        for (word, actual) in failures.iter().take(50) {
            eprintln!("{:<20} {:<20}", word, actual);
        }
    }

    // CI threshold: English words should mostly pass through unchanged.
    // The engine transforms English words that look like valid Vietnamese
    // Telex sequences (e.g. "reset" → "rết", "seen" → "sên"). This is
    // expected behavior for a Vietnamese IME — there's no way to distinguish
    // "reset" from a Vietnamese Telex input without a dictionary.
    // 93% reflects the realistic trade-off between Vietnamese coverage
    // and English passthrough.
    const MIN_PASS_RATE: f64 = 93.0;
    assert!(
        pass_rate >= MIN_PASS_RATE,
        "English passthrough rate {:.2}% is below threshold {:.1}%",
        pass_rate,
        MIN_PASS_RATE
    );
}
