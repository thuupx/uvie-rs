use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use uvie::{InputMethod, UltraFastViEngine};
use vi::methods::transform_buffer as vi_transform_buffer;

fn type_seq(engine: &mut UltraFastViEngine, seq: &str) {
    engine.clear();
    for c in seq.chars() {
        black_box(engine.feed(c));
    }
}

fn type_seq_vi(def: &vi::Definition, out: &mut String, seq: &str) {
    out.clear();
    vi_transform_buffer(def, seq.chars(), out);
    black_box(&out);
}

fn bench_uvie_telex(c: &mut Criterion) {
    let mut group = c.benchmark_group("uvie_telex");

    let cases: &[(&str, &str)] = &[
        ("simple", "phoos"),
        ("sentence", "Tooi ddang gox Tieengs Vieejt " ),
        ("mixed", "clear free pro "),
        ("uow", "huows"),
        ("cluster", "nghees"),
    ];

    for (name, seq) in cases {
        group.bench_with_input(BenchmarkId::from_parameter(*name), seq, |b, input| {
            let mut e = UltraFastViEngine::new();
            e.set_input_method(InputMethod::Telex);
            b.iter(|| {
                type_seq(&mut e, input);
            })
        });
    }

    group.finish();
}

fn bench_compare_telex(c: &mut Criterion) {
    let mut group = c.benchmark_group("compare_telex");

    let cases: &[(&str, &str)] = &[
        ("simple", "phoos"),
        ("sentence", "Tooi ddang gox Tieengs Vieejt "),
        ("mixed", "clear free pro "),
        ("uow", "huows"),
        ("cluster", "nghees"),
        ("ui", "guiwr tuis"),
    ];

    for (name, seq) in cases {
        group.bench_with_input(BenchmarkId::new("uvie", *name), seq, |b, input| {
            let mut e = UltraFastViEngine::new();
            e.set_input_method(InputMethod::Telex);
            b.iter(|| {
                type_seq(&mut e, input);
            })
        });

        group.bench_with_input(BenchmarkId::new("vi", *name), seq, |b, input| {
            let mut out = String::new();
            b.iter(|| {
                type_seq_vi(&vi::TELEX, &mut out, input);
            })
        });
    }

    group.finish();
}

fn bench_compare_vni(c: &mut Criterion) {
    let mut group = c.benchmark_group("compare_vni");

    let cases: &[(&str, &str)] = &[
        ("simple", "pho61"),
        ("sentence", "Tooi2 dang5 go6 Tie6ng2 Vie6t5 "),
        ("mixed", "clear free pro "),
        ("cluster", "nghe61"),
        ("ui", "guiw0 tui1"),
    ];

    for (name, seq) in cases {
        group.bench_with_input(BenchmarkId::new("uvie", *name), seq, |b, input| {
            let mut e = UltraFastViEngine::new();
            e.set_input_method(InputMethod::Vni);
            b.iter(|| {
                type_seq(&mut e, input);
            })
        });

        group.bench_with_input(BenchmarkId::new("vi", *name), seq, |b, input| {
            let mut out = String::new();
            b.iter(|| {
                type_seq_vi(&vi::VNI, &mut out, input);
            })
        });
    }

    group.finish();
}

fn bench_uvie_vni(c: &mut Criterion) {
    let mut group = c.benchmark_group("uvie_vni");

    let cases: &[(&str, &str)] = &[
        ("simple", "pho61"),
        ("sentence", "Tooi2 dang5 go6 Tie6ng2 Vie6t5 "),
        ("mixed", "clear free pro "),
        ("uow_like", "huo71"),
        ("cluster", "nghe61"),
    ];

    for (name, seq) in cases {
        group.bench_with_input(BenchmarkId::from_parameter(*name), seq, |b, input| {
            let mut e = UltraFastViEngine::new();
            e.set_input_method(InputMethod::Vni);
            b.iter(|| {
                type_seq(&mut e, input);
            })
        });
    }

    group.finish();
}

// Placeholder for "vi-rs" comparison.
// Once you provide the crates.io package name + the API to feed characters, we can add:
// - a dev-dependency to that crate
// - a bench group that runs the same input cases

criterion_group!(
    benches,
    bench_uvie_telex,
    bench_uvie_vni,
    bench_compare_telex,
    bench_compare_vni
);
criterion_main!(benches);
