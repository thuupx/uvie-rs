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
