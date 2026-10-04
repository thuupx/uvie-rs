use uvie::UltraFastViEngine;
use uvie::diff::Diffable;

fn type_diff(engine: &mut UltraFastViEngine, input: &str) -> String {
    let mut screen = String::new();
    for c in input.chars() {
        if c.is_whitespace() {
            engine.commit();
            screen.push(c);
        } else {
            let (bs, suffix) = engine.feed_diff(c);
            for _ in 0..bs {
                screen.pop();
            }
            screen.push_str(suffix);
        }
    }
    screen
}

fn main() {
    let data = include_str!("../tests/data/english_100k.txt");
    let mut fails: Vec<(String, String)> = Vec::new();
    for line in data.lines() {
        let word = line.trim();
        if word.is_empty() || word.starts_with('#') {
            continue;
        }
        let mut e = UltraFastViEngine::new();
        e.set_modern_orthography(false);
        let typed = format!("{} ", word);
        let result = type_diff(&mut e, &typed);
        let actual = result.trim();
        if actual != word {
            fails.push((word.to_string(), actual.to_string()));
        }
    }
    println!("total failures (feed_diff): {}", fails.len());
    use std::io::Write;
    let mut f = std::fs::File::create("/tmp/diff_fails.txt").unwrap();
    for (w, a) in &fails {
        writeln!(f, "{}\t{}", w, a).unwrap();
    }
    // breakdown by length
    let mut by_len = std::collections::BTreeMap::new();
    for (w, _) in &fails {
        *by_len.entry(w.len()).or_insert(0) += 1;
    }
    for (l, c) in by_len {
        println!("len {}: {}", l, c);
    }
}
