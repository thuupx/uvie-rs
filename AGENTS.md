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

## Performance Results (cumulative, 3 optimization rounds)

| Benchmark | Original | After Round 1 | After Round 2 | After Round 3 | Total |
|-----------|----------|---------------|---------------|---------------|-------|
| diff_short/phoos | 569 ns | 371 ns | 336 ns | 307 ns | **-46%** |
| diff_workaround/chuaw | 637 ns | 458 ns | 407 ns | 374 ns | **-41%** |
| diff_workaround/ngieengx | 1.18 µs | 814 ns | 733 ns | 674 ns | **-43%** |
| diff_sentences/long | 10.2 µs | 7.18 µs | 6.41 µs | 5.99 µs | **-41%** |
| diff_backspace/medium | 4.70 µs | 3.13 µs | 2.62 µs | 2.41 µs | **-49%** |
| diff_backspace/long | 4.54 µs | 3.11 µs | 2.63 µs | 2.41 µs | **-47%** |

### Round 1: Eliminate heap allocations
- `diff_into`: single-pass char counting (was double `.chars().count()`)
- `feed_diff_core`: `mem::swap`/`mem::take` instead of `String::clone`
- `handle_telex_w`: snapshot only changed Syl entry (was cloning 192B SylBuf)
- `rerender_chars`: `thread_local` scratch engine (was allocating new engine)

### Round 2: Stack-only OutBuffer + partition cache
- `StackStr<N>`: `[u8; N]` UTF-8 buffer with `Deref<Target=str>` via
  `from_utf8_unchecked` — replaces `String`, zero heap
- `SylBuf.version()`: increments on every mutation, enables O(1) cache
  validity check for `partition_syllable()`
- `partition_syllable()` cache: avoids 5-10 redundant O(n) scans per keystroke

### Round 3: Branch prediction
- `feed_diff_core`: check cheap conditions first before touching scratch buffers
- Skip `scratch_display` build on common path (not optimistic)
- Optimistic display only built when `is_optimistic` is true (rare)

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
