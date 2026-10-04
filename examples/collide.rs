//! Find English-dict candidates that shadow real Vietnamese typing
//! sequences: any candidate that is an initial prefix (len>=4) of a
//! Telex keystroke sequence from the 30k pairs file hijacks that word
//! mid-typing (per-keystroke override + sticky passthrough).
use std::collections::HashSet;

fn main() {
    let pairs = include_str!("/Users/devin/repos/uvie-rs/tests/data/vietnamese_telex_pairs.txt");
    let mut inputs: Vec<String> = Vec::new();
    for line in pairs.lines() {
        if let Some(inp) = line.split_whitespace().next() {
            inputs.push(inp.to_string());
        }
    }
    println!("pair inputs: {}", inputs.len());

    // Candidate dict = current union file on disk (one word per line).
    let dict_data = std::fs::read_to_string("/tmp/union_dict.txt").unwrap();
    let dict: HashSet<String> = dict_data.lines().map(|l| l.trim().to_string()).collect();
    println!("dict: {}", dict.len());

    // A candidate shadows a Vi sequence iff the sequence starts with it.
    // Since dict is sorted-ish small vs 30k inputs, index inputs by their
    // first-4 chars to prune.
    let mut shadowed: Vec<(String, Vec<String>)> = Vec::new();
    for w in &dict {
        if w.len() < 4 { continue; }
        let mut hits: Vec<String> = Vec::new();
        for inp in &inputs {
            if inp.len() >= w.len() && inp.starts_with(w.as_str()) {
                hits.push(inp.clone());
            }
        }
        if !hits.is_empty() {
            shadowed.push((w.clone(), hits));
        }
    }
    shadowed.sort_by_key(|(w, _)| w.clone());
    println!("shadowing candidates: {}", shadowed.len());
    for (w, hits) in &shadowed {
        let shown: Vec<&str> = hits.iter().take(6).map(|s| s.as_str()).collect();
        println!("  {} -> {:?}", w, shown);
    }
}
