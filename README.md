# uvie

A really fast Vietnamese input method engine in Rust.

## Why it is "ultra fast"

Benchmarks (`cargo bench`) usually put `uvie` in the **ns → low µs** range per sequence.

- **`ns`** = nanoseconds, **`µs`** = microseconds (1 µs = 1000 ns)

Why it’s fast:

- **Zero allocation** (No String allocations)
- **Modern algorithms** (no extra scans, no extra passes)
- **Small fixed buffers** (cache-friendly, predictable)
- **Table lookups & bitmasks** instead of big branchy code
- **Minimal allocation** (optional `heapless` mode)

- Supports **Telex** and **VNI** input methods.
- Optimized for tight loops (fixed-size buffers, table-driven mappings).
- Can be built in a heapless-friendly configuration for embedded/low-resource environments.

## Features

- **Easy to use**: simple API, no dependencies, easy to embed, extensible.
- **Default (`std`)**: normal Rust `String` buffers.
- **`heapless`**: uses fixed-capacity `heapless::String` buffers (no heap allocation from the engine itself).

> Note: in `heapless` mode, if internal buffers overflow, output may be truncated.

## How to use

Add it to your project:

```toml
[dependencies]
uvie = { path = "../uvie" }
```

Telex:

```rust
use uvie::{InputMethod, UltraFastViEngine};

let mut e = UltraFastViEngine::new();
e.set_input_method(InputMethod::Telex);

for ch in "phoos".chars() {
    e.feed(ch);
}
assert_eq!(e.feed(' '), "phố ");
```

VNI:

```rust
use uvie::{InputMethod, UltraFastViEngine};

let mut e = UltraFastViEngine::new();
e.set_input_method(InputMethod::Vni);

for ch in "viet65".chars() {
    e.feed(ch);
}
assert_eq!(e.feed(' '), "việt ");
```

CLI demo:

```bash
cargo run -- --mode telex
cargo run -- --mode vni
```

Embedded/heapless check:

```bash
cargo check --no-default-features --features heapless
```

## CLI demo

The repository contains a small interactive CLI (enabled only with `std`).

```bash
cargo run -- --mode telex
cargo run -- --mode vni
```

Controls:

- Press `Enter` to flush (it feeds a space)
- Press `Ctrl+C` to exit

## Benchmarks (uvie vs vi)

Benchmarks use `criterion`.

```bash
cargo bench
```

`cargo bench` runs benchmarks in the `bench` profile (optimized / release-like version).

The benchmark file is in `benches/perf.rs` and currently benchmarks:

- `uvie_telex`
- `uvie_vni`

It also includes direct comparisons against the [`vi`](https://docs.rs/vi/latest/vi/) crate:

- `compare_telex/*`
- `compare_vni/*`

### Fairness notes

- `uvie` is benchmarked by reusing a single `UltraFastViEngine` instance per benchmark and calling `clear()` between iterations.
- `vi` is benchmarked via `vi::methods::transform_buffer`, reusing a single `String` output buffer per benchmark iteration.

### Results

The exact numbers depend on CPU/OS, but the ratio is stable.

Sample run (Apple Silicon, `cargo bench`):

| Case | Telex speedup (vi / uvie) | VNI speedup (vi / uvie) |
| --- | ---: | ---: |
| simple | ~16x | ~18x |
| sentence | ~15x | ~14x |
| mixed | ~17x | ~11x |
| uow / uow_like | ~15x | n/a |
| cluster | ~15x | ~15x |
| ui | ~22x | ~11x |

See more details at [online report](https://thuupx.github.io/uvie-rs/criterion/report/)

## Embedded / heapless build

To build the library without default `std` and with heapless buffers:

```bash
cargo check --no-default-features --features heapless
```
