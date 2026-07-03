use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use uvie::diff::Diffable;
use uvie::{InputMethod, UltraFastViEngine};
use vi::methods::transform_buffer as vi_transform_buffer;

// ---------------------------------------------------------------------------
// Helpers — the app uses `feed_diff` (the diff API), not the raw `feed` API.
// ---------------------------------------------------------------------------

fn type_seq_diff(engine: &mut UltraFastViEngine, seq: &str) {
    engine.reset_diff();
    for c in seq.chars() {
        black_box(engine.feed_diff(c));
    }
}

fn type_seq_vi(def: &vi::Definition, out: &mut String, seq: &str) {
    out.clear();
    vi_transform_buffer(def, seq.chars(), out);
    black_box(&out);
}

// ---------------------------------------------------------------------------
// Test cases
// ---------------------------------------------------------------------------

const SHORT_WORDS: &[(&str, &str)] = &[
    ("phoos", "phoos"),
    ("huows", "huows"),
    ("nghees", "nghees"),
    ("ddoans", "ddoans"),
    ("choas", "choas"),
    ("tuis", "tuis"),
    ("quys", "quys"),
    ("dduwowcj", "dduwowcj"),
];

const WORKAROUND_WORDS: &[(&str, &str)] = &[
    ("chuaw", "chuaw"),
    ("nguoowcj", "nguoowcj"),
    ("dduwocj", "dduwocj"),
    ("hieej", "hieej"),
    ("ngieengx", "ngieengx"),
    ("khoajch", "khoajch"),
    ("ddoansj", "ddoansj"),
    ("uyeer", "uyeer"),
    ("nhieept", "nhieept"),
    ("thuyeest", "thuyeest"),
];

const LONG_SENTENCES: &[(&str, &str)] = &[
    ("sentence_short", "Tooi ddang gox Tieengs Vieejt "),
    (
        "sentence_medium",
        "Tooi ddang gox Tieengs Vieejt baengs boox gox UVieKey ",
    ),
    (
        "sentence_long",
        "Tooi ddang gox Tieengs Vieejt baengs boox gox UVieKey vaex noos rraats nhahj vaaf chinhx xacs ",
    ),
    (
        "sentence_mixed",
        "Hello Tooi ddang gox Tieengs Vieejt, clear free pro ",
    ),
    (
        "sentence_workaround",
        "Nguyieenx Tuis ddang gox ngieengx ddieeuw nhieept thuyeest ",
    ),
];

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_diff_short(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_short");
    for (name, seq) in SHORT_WORDS {
        group.bench_with_input(BenchmarkId::from_parameter(*name), seq, |b, input| {
            let mut e = UltraFastViEngine::new();
            e.set_input_method(InputMethod::Telex);
            b.iter(|| type_seq_diff(&mut e, input));
        });
    }
    group.finish();
}

fn bench_diff_workaround(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_workaround");
    for (name, seq) in WORKAROUND_WORDS {
        group.bench_with_input(BenchmarkId::from_parameter(*name), seq, |b, input| {
            let mut e = UltraFastViEngine::new();
            e.set_input_method(InputMethod::Telex);
            b.iter(|| type_seq_diff(&mut e, input));
        });
    }
    group.finish();
}

fn bench_diff_sentences(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_sentences");
    for (name, seq) in LONG_SENTENCES {
        group.bench_with_input(BenchmarkId::from_parameter(*name), seq, |b, input| {
            let mut e = UltraFastViEngine::new();
            e.set_input_method(InputMethod::Telex);
            b.iter(|| type_seq_diff(&mut e, input));
        });
    }
    group.finish();
}

fn bench_diff_backspace(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_backspace");

    let cases: &[(&str, &str)] = &[
        ("short", "phoos"),
        ("medium", "dduwowcj"),
        ("long", "nguoowcj"),
        ("workaround", "ngieengx"),
    ];

    for (name, seq) in cases {
        group.bench_with_input(BenchmarkId::from_parameter(*name), seq, |b, input| {
            let mut e = UltraFastViEngine::new();
            e.set_input_method(InputMethod::Telex);
            b.iter(|| {
                type_seq_diff(&mut e, input);
                let len = input.chars().count();
                for _ in 0..len {
                    black_box(e.backspace_diff());
                }
            });
        });
    }
    group.finish();
}

fn bench_compare_telex(c: &mut Criterion) {
    let mut group = c.benchmark_group("compare_telex");

    let cases: &[(&str, &str)] = &[
        ("simple", "phoos"),
        ("sentence", "Tooi ddang gox Tieengs Vieejt "),
        ("workaround", "ngieengx"),
    ];

    for (name, seq) in cases {
        group.bench_with_input(BenchmarkId::new("uvie", *name), seq, |b, input| {
            let mut e = UltraFastViEngine::new();
            e.set_input_method(InputMethod::Telex);
            b.iter(|| type_seq_diff(&mut e, input));
        });

        group.bench_with_input(BenchmarkId::new("vi", *name), seq, |b, input| {
            let mut out = String::new();
            b.iter(|| type_seq_vi(&vi::TELEX, &mut out, input));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_diff_short,
    bench_diff_workaround,
    bench_diff_sentences,
    bench_diff_backspace,
    bench_compare_telex,
);
criterion_main!(benches);
