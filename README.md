# uvie-rs

Ultra-fast Vietnamese input method engine (Telex, VNI) written in Rust.

A `no_std` / `no-alloc` compatible library with **zero external dependencies**. Designed for sub-microsecond latency per keystroke through incremental state updates, positive syllable validation, and 100% stack-allocated hot paths.

**macOS implementation**: See [UVieKey](https://github.com/thuupx/UVieKey) for the macOS menu bar app that uses this engine.

## Features

- **Telex & VNI**: Full support for both popular input methods.
- **Modern orthography**: Optional tone placement per new standard (e.g. `hoas` → `hoá`).
- **Relaxed coda mode**: Optional lenient coda validation for flexible typing.
- **Per-character state**: Each keystroke gets its own state entry; transforms are bit-flips, not multi-pass reordering.
- **Validate raw keystrokes**: Checks raw ASCII sequence against positive syllable tables before any transform. English passthrough is automatic.
- **Diff-based API**: Returns `(backspace_count, suffix_to_type)` per keystroke for minimal screen updates.
- **O(1) backspace**: Snapshot stack mechanism eliminates O(n²) rebuild on backspace.
- **No heap in hot path**: All buffer types (`StackStr`, `CharVec`, `SylBuf`) are stack-allocated — zero allocation per keystroke.
- **Zero dependencies**: No `heapless`, no external crates — pure Rust, `no_std`-compatible out of the box.
- **`no_std` compatible**: Works in embedded or constrained environments with `--no-default-features`.

## Architecture

The engine uses an incremental, stateful model with per-character state buffers. See [`src/ARCHITECT.md`](src/ARCHITECT.md) for detailed design rationale, state model, and per-keystroke flow.

## Usage

```rust
use uvie::{UltraFastViEngine, InputMethod};

let mut engine = UltraFastViEngine::new();
engine.set_input_method(InputMethod::Telex);

// Feed keystrokes
let result = engine.feed('t');  // "t"
let result = engine.feed('i');  // "ti"
let result = engine.feed('e');  // "tie"
let result = engine.feed('s');  // "tiế"

// Commit word (e.g. on space)
engine.commit();
```

## FFI / C API

The library compiles to `staticlib` and `cdylib`. The C API provides:

- `uvie_engine_new()` / `uvie_engine_free()`
- `uvie_feed(engine, key, &backspace_count, suffix, suffix_len)`
- `uvie_backspace(engine, &backspace_count, suffix, suffix_len)`
- `uvie_set_mode(engine, method)`
- Configuration toggles (modern orthography, etc.)

See [`src/ffi.rs`](src/ffi.rs) for the full C API.

## Building

```bash
# Build release library
cargo build --release

# Run benchmarks
cargo bench

# Run tests
cargo test
```

For `no_std` builds:

```bash
cargo build --release --no-default-features
```

## Prebuilt releases

GitHub releases provide ready-to-link static libraries:

| Platform | Asset | Contents |
| -------- | ----- | -------- |
| macOS universal | `uvie-macos-universal.tar.gz` | `libuvie.a` (arm64 + x86_64), `uvie.h` |
| Linux x86_64 | `uvie-linux-x86_64.tar.gz` | `libuvie.a`, `uvie.h` |
| Windows x86_64 | `uvie-windows-x86_64.zip` | `uvie.lib`, `uvie.h` |

See the [release workflow](.github/workflows/release.yml) for build details.

## Benchmark

Apple Silicon (`cargo bench`), comparison against the `vi` crate:

| Case | Telex speedup (vi / uvie) | VNI speedup (vi / uvie) |
| ------ | --------------------------: | ------------------------: |
| simple | ~5.8x | ~5.7x |
| sentence | ~6.1x | ~5.3x |
| mixed | ~15.8x | ~10.7x |
| cluster | ~6.7x | ~6.7x |
| ui | ~5.8x | ~2.8x |

### Diff API performance (v2.1.0, after optimization rounds)

| Benchmark | Original | v2.1.0 | Improvement |
|-----------|----------|--------|-------------|
| diff_short / phoos | 569 ns | 307 ns | **-46%** |
| diff_workaround / chuaw | 637 ns | 374 ns | **-41%** |
| diff_workaround / ngieengx | 1.18 µs | 674 ns | **-43%** |
| diff_sentences / long | 10.2 µs | 5.99 µs | **-41%** |
| diff_backspace / medium | 4.70 µs | 2.41 µs | **-49%** |
| diff_backspace / long | 4.54 µs | 2.41 µs | **-47%** |

Optimizations: single-pass char counting, `mem::swap`/`mem::take` instead of
`String::clone`, `StackStr<N>` stack-allocated UTF-8 buffer, version-based
`partition_syllable()` cache, branch prediction reordering, O(1) backspace
via snapshot stack, and `thread_local` scratch engine for V-C-V splits.

Full report: [thuupx.github.io/uvie-rs/criterion/report/](https://thuupx.github.io/uvie-rs/criterion/report/)

## License

MIT OR Apache-2.0
