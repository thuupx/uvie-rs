//! Scenario-based benchmarks, following the per-scenario reporting format:
//!
//! | Scenario | What it measures |
//! |----------|------------------|
//! | Compound Word | deep onset + circumflex nucleus + coda + tone (`nghiengs`) |
//! | Random Keystroke Sequence | seeded random letters, unpredictable paths |
//! | Worst-case Deep Syllable | stroke + horn + tone in one syllable (`dduwowcj`) |
//! | Mixed Typing | Vietnamese Telex + English dict-override words |
//! | Rapid Backspace Burst | type a word, then backspace the whole word |
//! | English Passthrough | phonotactically invalid input rendered raw |
//! | Feed Benchmark | the legacy non-diff `feed()` API |
//!
//! One criterion iteration = one natural typing unit (a word, a sentence, or
//! a type+delete cycle). `Throughput::Elements` reports the keystroke count
//! so per-keystroke cost is comparable across scenarios. All diff scenarios
//! go through `feed_diff` — the API the Swift app drives over FFI.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use rand::prelude::*;
use rand::rngs::StdRng;
use uvie::diff::Diffable;
use uvie::{InputMethod, UltraFastViEngine};

fn telex_engine() -> UltraFastViEngine {
    let mut e = UltraFastViEngine::new();
    e.set_input_method(InputMethod::Telex);
    e
}

fn type_seq_diff(engine: &mut UltraFastViEngine, seq: &str) {
    engine.reset_diff();
    for c in seq.chars() {
        black_box(engine.feed_diff(c));
    }
}

/// Deterministic random keystroke sequence (seeded, generated once).
fn random_keystrokes(n: usize) -> String {
    let mut rng = StdRng::seed_from_u64(0x5556_4945);
    (0..n).map(|_| rng.gen_range(b'a'..=b'z') as char).collect()
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

/// "nghieengs" → "nghiếng": ngh onset + iê nucleus + ng coda + sắc tone.
fn bench_compound_word(c: &mut Criterion) {
    let mut group = c.benchmark_group("compound_word");
    let seq = "nghieengs ";
    group.throughput(Throughput::Elements(seq.chars().count() as u64));
    group.bench_with_input(BenchmarkId::from_parameter("nghieengs"), seq, |b, input| {
        let mut e = telex_engine();
        b.iter(|| type_seq_diff(&mut e, input));
    });
    group.finish();
}

/// 32 random lowercase letters + word-boundary space.
fn bench_random_keystroke_sequence(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_keystroke_sequence");
    let seq: String = random_keystrokes(32) + " ";
    group.throughput(Throughput::Elements(seq.chars().count() as u64));
    group.bench_with_input(BenchmarkId::from_parameter("seeded_32"), &seq, |b, input| {
        let mut e = telex_engine();
        b.iter(|| type_seq_diff(&mut e, input));
    });
    group.finish();
}

/// "dduwowcj" → "được": d-stroke + uơ horn + nặng tone in a single syllable.
fn bench_worst_case_deep_syllable(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case_deep_syllable");
    let seq = "dduwowcj ";
    group.throughput(Throughput::Elements(seq.chars().count() as u64));
    group.bench_with_input(BenchmarkId::from_parameter("dduwowcj"), seq, |b, input| {
        let mut e = telex_engine();
        b.iter(|| type_seq_diff(&mut e, input));
    });
    group.finish();
}

/// Vietnamese Telex interleaved with English words that trigger the
/// dictionary override ("character", "safari", "good", "book") and
/// phonotactic passthrough.
fn bench_mixed_typing(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_typing");
    let seq = "Hello Tooi ddang gox Tieengs Vieejt baengs boox gox UVieKey, \
               character safari good book clear free ";
    group.throughput(Throughput::Elements(seq.chars().count() as u64));
    group.bench_with_input(BenchmarkId::from_parameter("viet_english"), &seq, |b, input| {
        let mut e = telex_engine();
        b.iter(|| type_seq_diff(&mut e, input));
    });
    group.finish();
}

/// Type "dduwowcj" → "được", then burst-backspace the whole word.
fn bench_rapid_backspace_burst(c: &mut Criterion) {
    let mut group = c.benchmark_group("rapid_backspace_burst");
    let word = "dduwowcj";
    group.throughput(Throughput::Elements(word.chars().count() as u64 * 2));
    group.bench_with_input(BenchmarkId::from_parameter("burst"), word, |b, input| {
        let mut e = telex_engine();
        b.iter(|| {
            for c in input.chars() {
                black_box(e.feed_diff(c));
            }
            for _ in 0..input.chars().count() {
                black_box(e.backspace_diff());
            }
        });
    });
    group.finish();
}

/// "ghost": gh + o is phonotactically invalid → raw English passthrough.
fn bench_english_passthrough(c: &mut Criterion) {
    let mut group = c.benchmark_group("english_passthrough");
    let seq = "ghost ";
    group.throughput(Throughput::Elements(seq.chars().count() as u64));
    group.bench_with_input(BenchmarkId::from_parameter("ghost"), seq, |b, input| {
        let mut e = telex_engine();
        b.iter(|| type_seq_diff(&mut e, input));
    });
    group.finish();
}

/// The legacy non-diff `feed()` API (used by benchmarks as `type_seq`).
fn bench_feed(c: &mut Criterion) {
    let mut group = c.benchmark_group("feed");
    let seq = "nghiengs ";
    group.throughput(Throughput::Elements(seq.chars().count() as u64));
    group.bench_with_input(BenchmarkId::from_parameter("legacy"), seq, |b, input| {
        let mut e = telex_engine();
        b.iter(|| {
            e.clear();
            for c in input.chars() {
                black_box(e.feed(c));
            }
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_compound_word,
    bench_random_keystroke_sequence,
    bench_worst_case_deep_syllable,
    bench_mixed_typing,
    bench_rapid_backspace_burst,
    bench_english_passthrough,
    bench_feed,
);
criterion_main!(benches);
