# AGENTS.md — uvie-rs Engine

## Build & Test Commands

```bash
rtk cargo build --release          # Build release
rtk cargo test --release           # Run all tests (215 tests)
rtk cargo bench --bench perf -- --warm-up-time 1 --measurement-time 3  # Benchmarks
```

## Architecture

The engine has two APIs:
- **`feed()`** — legacy non-diff API (used by benchmarks as `type_seq`)
- **`feed_diff()`** — diff API (what the Swift app actually calls via FFI)
  - Returns `(backspaces, suffix)` — minimal edit instructions
  - Tracks `prev_rendered` (on-screen) vs `out_buf` (engine output)
  - V-C-V split: auto-commits first syllable when a vowel starts a new one

## Performance Hotspots (measured 2026-07-03)

### Bottleneck: String (OutBuffer) clones on diff path
The `std` build uses `OutBuffer = String` (heap). Every `feed_diff` call
was cloning Strings 3-5 times. Fixed by using `mem::swap`/`mem::take`
and scratch buffers in `DiffState`.

### Bottleneck: `handle_telex_w` SylBuf clones
Was cloning the entire SylBuf (24 × Syl = 192 bytes) per candidate.
Fixed by snapshotting only the changed entry + restoring on failure.

### Bottleneck: `rerender_chars` creating new engine
Was allocating a fresh `UltraFastViEngine` on every V-C-V split.
Fixed with a `thread_local` scratch engine.

### Bottleneck: `diff_into` double char counting
Was calling `.chars().count()` twice. Fixed with single-pass counting.

## Performance Results (after optimization)

| Benchmark | Before | After | Improvement |
|-----------|--------|-------|-------------|
| diff_short/phoos | 569 ns | 371 ns | -35% |
| diff_workaround/chuaw | 637 ns | 458 ns | -28% |
| diff_workaround/ngieengx | 1.18 µs | 814 ns | -31% |
| diff_sentences/long | 10.2 µs | 7.18 µs | -30% |
| diff_backspace/medium | 4.70 µs | 3.13 µs | -33% |

## Key Design Decisions

- `OutBuffer = String` for `std`, `heapless::String<128>` for `no_std`
- `SylBuf` is a fixed `[Syl; 24]` array (stack, no heap)
- `CharVec<24>` for raw keystroke tracking (stack, no heap)
- `partition_syllable()` is called multiple times per keystroke — it's
  O(n) where n ≤ 24, so it's fast but could be cached
- `is_valid_vietnamese()` calls `partition_syllable()` internally
- Traditional orthography: tone on first vowel for open syllables (hoá),
  second vowel for syllables with coda (hoạt, đoán)
- `apply_coda_tone_rule()` fixes tone placement at render time

## Benchmark Cases

- `diff_short` — simple words exercising modifier/tone paths
- `diff_workaround` — words with many if/else branches (V-C-V splits,
  double-vowel circumflex, w modifier with multiple candidates,
  mid-nucleus tone, tone + coda + reapply)
- `diff_sentences` — long sentences with word boundaries
- `diff_backspace` — type word then backspace all the way back
- `compare_telex` — uvie vs vi-rs crate comparison
