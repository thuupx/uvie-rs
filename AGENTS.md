# AGENTS.md — uvie-rs Engine

## Build & Test Commands

```bash
rtk cargo build --release          # Build release
rtk cargo test --release           # Run all tests (233 tests)
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

### Accuracy Improvement (2025-01)
Data-driven testing with 30k Vietnamese Telex pairs:
- **Before**: 89.42% pass rate (3211 failures)
- **After**: 99.98% pass rate (6 failures — rare edge cases like `huow`→`huơ`)

Key fixes:
- Allow `e+tone+e` after consonant onsets (`befe`→`bề`, `biecse`→`biếc`)
- `cleanup_literal_tone_after_circumflex`: remove tone keys stuck in coda
  after circumflex is applied to an intermediate nucleus
- `cleanup_coda_tone_keys`: remove tone keys stuck in coda after `w` modifier
  transforms an intermediate nucleus into a valid one
- Silent `w` consume when all candidates have horn (`chuwongw`) or no valid
  candidate exists (`buwouw`)
- Circumflex validity check in `handle_vowel`: revert if result is invalid
  (`khoafo`→`khoào` instead of passthrough)
- Added missing triphthongs: `uây`, `oeo`, `uêu`
- `apply_coda_tone_rule`: added `oo` diphthong, fixed `n>=3` check
- `F_LITERAL` passthrough: check `is_valid_vietnamese()` before `F_LITERAL`
  (fixes `booongs`→`boóng` after triple-cancel)

Trade-off: ~40% regression on short benchmarks (~130ns/keystroke) due to
added `is_valid_vietnamese()` checks. Sentence/backspace benchmarks
remain faster than original baseline.

### Accuracy Improvement (2026-09): orthography-gated V-C-V + rime table cleanup
Two changes grounded in the standard Quốc Ngữ rime inventory (Vietnamese
orthography; "Các vần trong tiếng Việt"):

1. **Hiatus rule in the V-C-V split** (`src/diff/core.rs`). A syllable
   boundary inside a written token must fall on a consonant (the next
   syllable's onset) — vowel-to-vowel hiatus never occurs inside a
   Vietnamese token; any V-V sequence must be a single nucleus. The split
   now only fires when `find_split_point` returns an index before the new
   vowel (`split < len - 1`). Fixes `ressearch`→`rếarch` (the split
   resurrected the double-cancelled sắc as a committed `rế` syllable) and
   `theeo`→`thêo`. Consonant splits (`neebo`→`nêbo`, `toocaa`→`tôcâ`) are
   unchanged.

2. **Removed non-standard "(rare)" nuclei** (`src/tables/nucleus.rs`):
   `êo`, `ôu`, `ơu`, `ưo`, `io`, `ău`, `ăy` — zero attestations in the 22k
   word list and absent from the standard rime whitelist. These let English
   input render as fake Vietnamese (`theeo`→`thêo`, `keeo`→`kêo`,
   `tawuf`→`tầu`). Transient states (`uu`→ưu, `uo`→ươ precursor, `âo` for
   `naaos`→nấo, `oo` engine extension) are kept. Note: standard Telex maps
   `auw`→`âu` (UniKey-compatible); the engine currently consumes the `w`
   and passes through `au` — a possible future compatibility improvement.

Tests: `tests/orthography_tests.rs` (hiatus, passthrough, locked V-C-V).
Updated locked expectations in `bugfix_tests.rs` (`keeo`→passthrough),
`vcv_tests.rs` (`auw`→`au`), `word_boundary_tests.rs` (`dauw`→`dau`).

### Accuracy Improvement (2026-08)
Onset↔nucleus distribution check (`onset_nucleus_compatible` in
`src/tables/onset.rs`). Vietnamese has a hard complementary distribution
for the palatal/velar series — `c`/`k`/`q` and `g`/`gh`/`ng`/`ngh`:
- `gh`/`ngh` only before `i`/`e`/`ê`
- `k` only before `i`/`e`/`ê`/`y`; `c` only before `a`/`ă`/`â`/`o`/`ô`/`ơ`/`u`/`ư`
- `g` before back vowels + `i` (standalone `gi`); `ng` before back vowels
- `q` only as the `qu` glide (length-1 `q` with a nucleus is invalid)

Without this, `is_valid_vietnamese` accepted e.g. `gh`+`o` as valid, so a
tone key inside an English word produced invalid Vietnamese such as
`ghost`→`ghót`. Now `gh`+`o` is rejected → `ghost` passes through. Wired
into both `is_valid_vietnamese` (`src/validation.rs`) and
`is_legal_syllable` (`src/tables/mod.rs`). No Vietnamese regression
(Telex pairs 99.98%, 22k round-trip 99.29% unchanged); English passthrough
improved 94.45%→94.63%.

Note: English words that produce *phonotactically valid* Vietnamese
syllables (e.g. `character`→`chẩcter`, `safari`→`sầri`, `reset`→`rết`)
cannot be passed through without a dictionary — the cross-tone
double-vowel circumflex path (`a`+tone+`a`→`â`) is required for valid
Vietnamese (`befe`→`bề`, `sầm` via `safam`). Restricting it regresses 851
Telex pairs. A dictionary-based check is the only clean fix.

### English Dictionary Override (2026-08)
`src/tables/english.rs` — 2056 common English words that produce garbled
Vietnamese+English hybrids (e.g. `character`→`chẩcter`, `safari`→`sầri`,
`good`→`gôd`, `book`→`bôk`). Sorted static `&[&str]` array with O(log n)
binary search — zero heap, zero dependencies, `no_std`-compatible.

The override fires **per-keystroke** (not just at word boundaries): as
soon as `word_raw` matches a dictionary word, the engine shows the raw
English word instead of the garbled Vietnamese transform. This means the
user sees "character" (not "chẩcter") while typing, before pressing space.

**Critical: state sync after override.** When the override fires, the
English word is committed to `diff_committed` and ALL composing state is
cleared (`raw_chars`, `key_log`, `buf`, `out_buf`, `prev_rendered`, etc.).
`word_raw` is preserved for future dict checks. This prevents ghost
characters when the user continues typing past a dict word (e.g. "good" →
"goodness"): without this, the V-C-V split would re-render the committed
portion from `raw_chars`, producing Vietnamese transforms ("gô") instead
of the English word ("good").

The diff API tracks a lossless `word_raw: CharVec<24>` buffer in
`DiffState` that survives V-C-V splits and double-tone-cancel (unlike
the lossy `raw_chars`). At each keystroke, if `is_english_override
(&word_raw)` returns true, the engine recomputes the diff from the
pre-keystroke screen to the raw English word, then commits the word and
clears composing state.

**Excluded** from the dictionary:
- Words whose transform is a real Vietnamese word (from 22k word list):
  `chaos`→`cháo`, `most`→`mót`, `boots`→`bốt`, `deeds`→`đế`
- Words whose V-C-V split components are both valid Vietnamese words:
  `user`→`u`+`sẻ`, `banana`→`bân`+`na`

Backspace in override state: after the override, the English word is in
`diff_committed` and `prev_rendered` is empty. Backspace pops from
`diff_committed` (and `word_raw` simultaneously) via the existing
`key_log.is_empty()` fallback path. No special override check needed.

Performance: the per-keystroke clones (`committed_before`, `prev_before`)
are gated by `word_raw.len() >= 4` (minimum dictionary word length),
so the common path (1-3 chars typed) has zero overhead. The dictionary
binary search is O(log 2056) ≈ 11 comparisons. Benchmarks show no
significant change on short benchmarks, -1.3% to -1.7% improvement on
sentence benchmarks, and -6.8% to -8.1% improvement on backspace
benchmarks (the old backspace override check is now dead code).

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

## Coda Shorthands (teen code)

The engine accepts two kinds of final-consonant shorthands, both rendered
**verbatim** (the typed char is kept, not converted):

| Shorthand | Stands for | Mode | Tone rule |
|-----------|------------|------|-----------|
| `g`       | `ng`       | `--relaxed-coda` toggle | any tone (like ng) |
| `h`       | `nh`       | `--relaxed-coda` toggle | any tone (like nh) |
| `k`       | `c`        | **always active**       | sắc/nặng only (like c) |
| `nk`      | `nh`       | **always active**       | any tone (like nh) |

The `k` / `nk` shorthands are always on (no toggle) to support common
teen code and province spellings: "đắk", "Đăk Lak", "đỉnk" (= đỉnh).
Implementation lives in `src/tables/coda.rs` (`is_legal_coda`,
`tone_allowed_for_coda`) — validation only, no rendering conversion.

## Benchmark Cases

- `diff_short` — simple words exercising modifier/tone paths
- `diff_workaround` — words with many if/else branches (V-C-V splits,
  double-vowel circumflex, w modifier with multiple candidates,
  mid-nucleus tone, tone + coda + reapply)
- `diff_sentences` — long sentences with word boundaries
- `diff_backspace` — type word then backspace all the way back
- `compare_telex` — uvie vs vi-rs crate comparison
