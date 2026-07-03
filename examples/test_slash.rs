use uvie::{UltraFastViEngine, diff::Diffable};

fn type_seq_diff(e: &mut UltraFastViEngine, s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        let (bs, suffix) = e.feed_diff(ch);
        for _ in 0..bs { out.pop(); }
        out.push_str(suffix);
    }
    out
}

fn main() {
    // Check pre-existing behaviour: does "windows" alone also produce ưindows?
    let mut e = UltraFastViEngine::new();
    println!("windows => {}", type_seq_diff(&mut e, "windows"));

    // And after space (which was already a word boundary before my fix)
    let mut e = UltraFastViEngine::new();
    println!(" windows => {}", type_seq_diff(&mut e, " windows"));
}
