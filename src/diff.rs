//! Diff-based input API and V-C-V syllable splitting.
//!
//! The diff engine wraps the core composing engine and computes minimal
//! (backspace_count, suffix_to_type) instructions for each keystroke.

use crate::buffers::{CharVec, OutBuffer, new_out_buffer};
use crate::composing::Composable;
use crate::engine::UltraFastViEngine;
use crate::modes::{IS_TONE_KEY, Mode};
use crate::syllable::SylBuf;

/// Snapshot of the composing state after a keystroke, used for O(1) backspace.
/// All fields are stack-allocated (no heap).
#[derive(Clone)]
pub struct ComposingSnapshot {
    pub buf: SylBuf,
    pub raw_len: usize,
    pub raw_chars: CharVec<24>,
    pub key_log: CharVec<24>,
    pub prev_rendered: OutBuffer,
    pub prev_inner_render: OutBuffer,
    pub last_valid_raw_len: usize,
    pub last_valid_coda_start: usize,
    pub last_valid_out: OutBuffer,
}

/// Diff-mode state: tracks what's on screen vs what the engine produced.
pub struct DiffState {
    /// Raw keystroke buffer for feed_diff (chars, not bytes; needed for V-C-V split).
    /// This is the "live" buffer used by the forward path; it is mutated by
    /// double-tone-cancel (swap+truncate) and V-C-V split (replaced by the new
    /// syllable), so it is NOT a faithful record of what the user typed.
    pub raw_chars: CharVec<24>,
    /// Lossless keystroke log for the current *composing* portion only.
    /// Unlike `raw_chars`, this is NEVER truncated/swap'd on double-tone-cancel,
    /// so backspace can faithfully rebuild the composing state by replaying it.
    /// It is reset on word boundary, full-buffer, V-C-V split (set to the new
    /// syllable, mirroring `raw_chars`), commit and reset.
    pub key_log: CharVec<24>,
    /// Composing text currently visible on screen (for diffing).
    pub prev_rendered: OutBuffer,
    /// The inner engine's true last render (diff baseline).
    pub prev_inner_render: OutBuffer,
    /// Raw char count at the last valid (non-passthrough) Vietnamese render.
    pub last_valid_raw_len: usize,
    /// Coda start index at the last valid Vietnamese render (used to avoid optimistic display when the syllable already has a coda).
    pub last_valid_coda_start: usize,
    /// Output at the last valid Vietnamese render, used for V-C-V split.
    pub last_valid_out: OutBuffer,
    /// Accumulated auto-committed text from V-C-V splits (diff mode only).
    pub diff_committed: OutBuffer,
    /// Scratch buffer backing the &str returned by feed_diff/backspace_diff.
    pub diff_suffix: OutBuffer,
    /// Scratch buffer for optimistic candidate (avoids heap alloc per keystroke).
    pub scratch_optimistic: OutBuffer,
    /// Scratch buffer for display_composed (avoids heap alloc per keystroke).
    pub scratch_display: OutBuffer,
    /// Snapshot stack: one entry per keystroke, enabling O(1) backspace
    /// instead of O(n) replay. Cleared on word boundary, commit, V-C-V split.
    pub snapshots: [Option<ComposingSnapshot>; 24],
    pub snapshot_count: usize,
}

impl Default for DiffState {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffState {
    pub const fn new() -> Self {
        Self {
            raw_chars: CharVec::new(),
            key_log: CharVec::new(),
            prev_rendered: OutBuffer::new(),
            prev_inner_render: OutBuffer::new(),
            last_valid_raw_len: 0,
            last_valid_coda_start: 0,
            last_valid_out: OutBuffer::new(),
            diff_committed: OutBuffer::new(),
            diff_suffix: OutBuffer::new(),
            scratch_optimistic: OutBuffer::new(),
            scratch_display: OutBuffer::new(),
            snapshots: [const { None }; 24],
            snapshot_count: 0,
        }
    }

    pub fn clear(&mut self) {
        self.raw_chars.clear();
        self.key_log.clear();
        self.prev_rendered.clear();
        self.prev_inner_render.clear();
        self.last_valid_raw_len = 0;
        self.last_valid_coda_start = 0;
        self.last_valid_out.clear();
        self.diff_committed.clear();
        self.diff_suffix.clear();
        self.scratch_optimistic.clear();
        self.scratch_display.clear();
        // Clear snapshots
        for s in &mut self.snapshots {
            *s = None;
        }
        self.snapshot_count = 0;
    }

    /// Push a snapshot of the current composing state.
    /// Called after each successful feed_diff_core.
    #[inline]
    pub fn push_snapshot(&mut self, snap: ComposingSnapshot) {
        if self.snapshot_count < 24 {
            self.snapshots[self.snapshot_count] = Some(snap);
            self.snapshot_count += 1;
        }
    }

    /// Pop and return the last snapshot, or None if empty.
    #[inline]
    pub fn pop_snapshot(&mut self) -> Option<ComposingSnapshot> {
        if self.snapshot_count == 0 {
            return None;
        }
        self.snapshot_count -= 1;
        self.snapshots[self.snapshot_count].take()
    }
}

/// Diff-mode input API: minimal-edit instructions for each keystroke.
pub trait Diffable {
    fn feed_diff(&mut self, ch: char) -> (usize, &str);
    fn backspace_diff(&mut self) -> (usize, &str);
    fn commit_diff(&mut self) -> (usize, &str);
    fn reset_diff(&mut self);
    fn is_composing_diff(&self) -> bool;
    fn current_composing_diff(&self) -> &str;
    fn committed_text_diff(&self) -> &str;
    fn prev_inner_render_debug(&self) -> &str;
    fn prev_rendered_debug(&self) -> &str;
}

impl Diffable for UltraFastViEngine {
    fn feed_diff(&mut self, ch: char) -> (usize, &str) {
        // Word boundary: commit, clear snapshots, return char directly.
        if Self::is_word_boundary(ch) {
            for s in &mut self.diff.snapshots {
                *s = None;
            }
            self.diff.snapshot_count = 0;
            let _ = self.diff.key_log.try_push(ch);
            return self.feed_diff_core(ch);
        }

        // Push snapshot of state BEFORE this keystroke (for O(1) backspace).
        // This captures the state that backspace should restore to.
        self.diff.push_snapshot(ComposingSnapshot {
            buf: self.buf.clone(),
            raw_len: self.raw_len,
            raw_chars: self.diff.raw_chars.clone(),
            key_log: self.diff.key_log.clone(),
            prev_rendered: self.diff.prev_rendered.clone(),
            prev_inner_render: self.diff.prev_inner_render.clone(),
            last_valid_raw_len: self.diff.last_valid_raw_len,
            last_valid_coda_start: self.diff.last_valid_coda_start,
            last_valid_out: self.diff.last_valid_out.clone(),
        });

        // Append to key_log and run the core pipeline.
        let _ = self.diff.key_log.try_push(ch);
        let (bs, _suffix) = self.feed_diff_core(ch);

        // Check if V-C-V split or full-buffer happened: key_log would be
        // shorter than snapshot_count (the pre-keystroke snapshots were
        // for the old, longer composing word). Reset snapshots — the first
        // backspace will use the O(n) replay fallback, which is correct
        // because the "before" state for the split vowel never existed
        // in the forward path.
        if self.diff.key_log.len() < self.diff.snapshot_count {
            for s in &mut self.diff.snapshots {
                *s = None;
            }
            self.diff.snapshot_count = 0;
        }

        (bs, &self.diff.diff_suffix)
    }

    fn backspace_diff(&mut self) -> (usize, &str) {
        // No composing keystrokes left → fall back to popping auto-committed text
        // (V-C-V split output) one rendered char at a time, matching the screen.
        if self.diff.key_log.is_empty() {
            if !self.diff.diff_committed.is_empty() {
                self.diff.diff_committed.pop();
                self.diff.diff_suffix.clear();
                return (1, &self.diff.diff_suffix);
            }
            self.diff.diff_suffix.clear();
            return (0, &self.diff.diff_suffix);
        }

        // O(1) fast path: pop snapshot and restore state directly.
        // The snapshot was pushed BEFORE the keystroke, so it contains the
        // state that backspace should restore to.
        if let Some(snap) = self.diff.pop_snapshot() {
            // Snapshot the on-screen text before restore.
            let prev = self.diff.prev_rendered.clone();

            // Restore engine state from snapshot.
            self.buf = snap.buf;
            self.raw_len = snap.raw_len;
            self.diff.raw_chars = snap.raw_chars;
            self.diff.key_log = snap.key_log;
            self.diff.prev_rendered = snap.prev_rendered.clone();
            self.diff.prev_inner_render = snap.prev_inner_render;
            self.diff.last_valid_raw_len = snap.last_valid_raw_len;
            self.diff.last_valid_coda_start = snap.last_valid_coda_start;
            self.diff.last_valid_out = snap.last_valid_out;

            // Diff old vs new composing text.
            let (bs, _) = Self::diff_into(
                &prev,
                &self.diff.prev_rendered,
                &mut self.diff.diff_suffix,
            );
            return (bs, &self.diff.diff_suffix);
        }

        // Fallback: O(n) replay path (used when snapshot stack is empty,
        // e.g. after V-C-V split or when snapshots were exhausted).
        let prev = self.diff.prev_rendered.clone();
        self.diff.key_log.pop();
        let log: CharVec<24> = self.diff.key_log.iter().copied().collect();
        self.rebuild_composing(&log);
        let new = self.diff.prev_rendered.clone();
        let (bs, _) = Self::diff_into(&prev, &new, &mut self.diff.diff_suffix);
        (bs, &self.diff.diff_suffix)
    }

    fn commit_diff(&mut self) -> (usize, &str) {
        self.buf.clear();
        self.raw_len = 0;
        self.out_buf.clear();
        self.diff.raw_chars.clear();
        self.diff.key_log.clear();
        self.diff.prev_rendered.clear();
        self.diff.prev_inner_render.clear();
        self.diff.last_valid_raw_len = 0;
        self.diff.last_valid_coda_start = 0;
        self.diff.last_valid_out.clear();
        // A word has just been finalised: the V-C-V auto-committed portion of
        // this word must not survive into the next word, otherwise it leaks onto
        // the following word as ghost characters and corrupts macro matching.
        self.diff.diff_committed.clear();
        self.diff.diff_suffix.clear();
        (0, &self.diff.diff_suffix)
    }

    fn reset_diff(&mut self) {
        self.buf.clear();
        self.raw_len = 0;
        self.out_buf.clear();
        self.diff.clear();
    }

    fn is_composing_diff(&self) -> bool {
        // Account for both the live composing keystrokes and any auto-committed
        // V-C-V text that is still on screen; otherwise the host can think the
        // engine is idle while committed text remains, skipping needed resets.
        !self.diff.key_log.is_empty() || !self.diff.diff_committed.is_empty()
    }

    fn current_composing_diff(&self) -> &str {
        &self.diff.prev_rendered
    }

    fn committed_text_diff(&self) -> &str {
        &self.diff.diff_committed
    }

    fn prev_inner_render_debug(&self) -> &str {
        &self.diff.prev_inner_render
    }

    fn prev_rendered_debug(&self) -> &str {
        &self.diff.prev_rendered
    }
}

// ---------------------------------------------------------------------------
// Core feed pipeline + backspace rebuild helpers.
//
// `feed_diff_core` is the forward pipeline shared by the public `feed_diff`
// (which appends to `key_log` first) and by `rebuild_composing` (which does
// not). Keeping the append out of the core lets backspace replay keystrokes
// without double-appending to the log.
// ---------------------------------------------------------------------------
impl UltraFastViEngine {
    pub(crate) fn feed_diff_core(&mut self, ch: char) -> (usize, &str) {
        // Word boundary: commit composing word, clear state, return char directly.
        if Self::is_word_boundary(ch) {
            self.buf.clear();
            self.raw_len = 0;
            self.out_buf.clear();
            self.diff.clear();
            let _ = self.diff.diff_suffix.push(ch);
            return (0, &self.diff.diff_suffix);
        }

        // Safety valve: buffer full - commit, start fresh.
        if self.diff.raw_chars.is_full() {
            self.render_out_buf();
            let _ = self.diff.diff_committed.push_str(&self.out_buf);
            let screen_before_len = self.diff.prev_rendered.chars().count();
            self.buf.clear();
            self.raw_len = 0;
            self.out_buf.clear();
            self.diff.raw_chars.clear();
            self.diff.key_log.clear();
            self.diff.prev_inner_render.clear();
            self.diff.last_valid_raw_len = 0;
            self.diff.last_valid_coda_start = 0;
            self.diff.last_valid_out.clear();
            let _ = self.diff.raw_chars.try_push(ch);
            let _ = self.diff.key_log.try_push(ch);
            self.feed(ch);
            // Swap out_buf into prev_rendered (zero-alloc).
            let new_composed = core::mem::take(&mut self.out_buf);
            let bs = screen_before_len;
            self.diff.diff_suffix.clear();
            let _ = self.diff.diff_suffix.push_str(&new_composed);
            self.diff.prev_rendered.clear();
            let _ = self.diff.prev_rendered.push_str(&new_composed);
            self.diff.prev_inner_render.clear();
            let _ = self.diff.prev_inner_render.push_str(&new_composed);
            return (bs, &self.diff.diff_suffix);
        }

        let raw_len_before = self.raw_len;
        let _ = self.diff.raw_chars.try_push(ch);
        self.feed(ch);
        let raw_len_after = self.raw_len;

        // Double-tone-cancel detection.
        // NOTE: `key_log` is intentionally NOT truncated here. It keeps the full
        // keystroke sequence (including the cancelling key) so that a later
        // backspace can replay it and re-trigger the cancel faithfully. Only
        // `raw_chars` (the live, lossy buffer) is swap+truncate'd.
        let double_cancel_fired =
            raw_len_after == raw_len_before && !self.diff.raw_chars.is_empty();
        if double_cancel_fired {
            let last_idx = self.diff.raw_chars.len() - 1;
            if last_idx >= 1 {
                self.diff.raw_chars.swap(last_idx - 1, last_idx);
                self.diff.raw_chars.truncate(last_idx);
                // Update engine's raw_len to match the modified diff.raw_chars
                self.raw_len = self.diff.raw_chars.len();
            }
            self.diff.last_valid_raw_len = 0;
            self.diff.last_valid_coda_start = 0;
            self.diff.last_valid_out.clear();
        }

        // Swap out_buf into new_composed (zero-alloc, avoids String::clone).
        let mut new_composed = new_out_buffer();
        core::mem::swap(&mut new_composed, &mut self.out_buf);
        let is_now_raw = Self::is_raw_passthrough_slice(&self.diff.raw_chars, &new_composed);

        if !is_now_raw {
            self.diff.last_valid_raw_len = self.diff.raw_chars.len();
            self.diff.last_valid_coda_start = Self::raw_coda_start(&self.diff.raw_chars);
            self.diff.last_valid_out.clear();
            let _ = self.diff.last_valid_out.push_str(&new_composed);
        }

        // Optimistic display: show coda consonant appended to valid Vietnamese.
        // Only use it when the valid Vietnamese syllable had no coda yet;
        // otherwise the screen and the engine's true state diverge, causing ghost characters.
        //
        // Branch prediction: is_optimistic is RARE (only when a single
        // consonant is appended after a valid Vietnamese syllable with no
        // coda). We check the cheap conditions first and short-circuit
        // before touching scratch buffers.
        let ch_is_tone = Self::is_tone_key_in_mode(ch, self.mode);

        // Fast check: if not raw or last_valid_out empty, skip optimistic entirely.
        let is_optimistic = is_now_raw
            && !ch_is_tone
            && !self.diff.last_valid_out.is_empty()
            && Self::is_single_consonant_appended_slice(
                &self.diff.raw_chars,
                self.diff.last_valid_raw_len,
            )
            && self.diff.last_valid_coda_start == self.diff.last_valid_raw_len;

        // Build display_composed: on the common path (not optimistic), we
        // diff directly from new_composed — no scratch buffer needed.
        // Only build scratch_display when optimistic (rare).
        let display_composed: &str = if is_optimistic {
            self.diff.scratch_display.clear();
            let _ = self.diff.scratch_display.push_str(&self.diff.last_valid_out);
            let _ = self.diff.scratch_display.push(ch);
            &self.diff.scratch_display
        } else {
            &new_composed
        };

        // Diff baseline: prev_rendered is used as-is (no clone needed — we
        // diff from it, then overwrite it below).
        self.diff.prev_inner_render.clear();
        let _ = self.diff.prev_inner_render.push_str(&new_composed);

        // V-C-V boundary detection.
        // RARE: only fires when a vowel starts a new syllable after a consonant.
        // Check cheap conditions first for branch prediction.
        let ch_is_vowel = Self::is_ascii_vowel(ch as u8);
        if is_now_raw
            && ch_is_vowel
            && !self.diff.last_valid_out.is_empty()
            && self.diff.last_valid_raw_len < self.diff.raw_chars.len()
        {
            let split = Self::find_split_point(&self.diff.raw_chars);
            if split > 0 {
                let committed_raw: CharVec<24> =
                    self.diff.raw_chars[..split].iter().copied().collect();
                let new_syl_raw: CharVec<24> =
                    self.diff.raw_chars[split..].iter().copied().collect();

                let committed_out = Self::rerender_chars(&committed_raw, self.mode);

                let _ = self.diff.diff_committed.push_str(&committed_out);

                // Restart engine with new syllable.
                self.buf.clear();
                self.raw_len = 0;
                self.out_buf.clear();
                for &c in new_syl_raw.iter() {
                    self.feed(c);
                }
                // Swap out_buf into new_composed2 (zero-alloc).
                let new_composed2 = core::mem::take(&mut self.out_buf);

                // Build full_screen in scratch_optimistic (reused buffer).
                self.diff.scratch_optimistic.clear();
                let _ = self.diff.scratch_optimistic.push_str(&committed_out);
                let _ = self.diff.scratch_optimistic.push_str(&new_composed2);
                let (bs, _) = Self::diff_into(
                    &self.diff.prev_rendered,
                    &self.diff.scratch_optimistic,
                    &mut self.diff.diff_suffix,
                );

                self.diff.raw_chars = new_syl_raw;
                // Mirror the composing portion into `key_log` so backspace can
                // rebuild exactly this syllable. The committed portion's
                // keystrokes are dropped from the log (its rendered form lives
                // on in `diff_committed`).
                self.diff.key_log = self.diff.raw_chars.iter().copied().collect();
                // CRITICAL FIX: Sync raw_len with diff.raw_chars after V-C-V split
                // Without this, backspace() will use wrong indices when replaying keystrokes
                self.raw_len = self.diff.raw_chars.len();
                self.diff.prev_rendered.clear();
                let _ = self.diff.prev_rendered.push_str(&new_composed2);
                self.diff.prev_inner_render.clear();
                let _ = self.diff.prev_inner_render.push_str(&new_composed2);
                let is_new_raw =
                    Self::is_raw_passthrough_slice(&self.diff.raw_chars, &new_composed2);
                if is_new_raw {
                    self.diff.last_valid_raw_len = 0;
                    self.diff.last_valid_coda_start = 0;
                    self.diff.last_valid_out.clear();
                } else {
                    self.diff.last_valid_raw_len = self.diff.raw_chars.len();
                    self.diff.last_valid_coda_start = Self::raw_coda_start(&self.diff.raw_chars);
                    self.diff.last_valid_out.clear();
                    let _ = self.diff.last_valid_out.push_str(&new_composed2);
                }
                return (bs, &self.diff.diff_suffix);
            }
        }

        // Normal path: diff from prev_rendered → display_composed.
        // No clone needed — we diff from prev_rendered then overwrite it.
        let (bs, _) = Self::diff_into(
            &self.diff.prev_rendered,
            display_composed,
            &mut self.diff.diff_suffix,
        );
        self.diff.prev_rendered.clear();
        let _ = self.diff.prev_rendered.push_str(display_composed);
        (bs, &self.diff.diff_suffix)
    }

    /// Rebuild the composing portion of the engine from a lossless keystroke log.
    ///
    /// Resets the core engine and the diff *composing* state, then replays `log`
    /// through `feed_diff_core`. `diff_committed` and `key_log` are intentionally
    /// preserved: the committed V-C-V prefix is still on screen, and `key_log`
    /// was already updated by the caller (`backspace_diff`).
    ///
    /// Because `log` is always a single composing syllable (the V-C-V split point
    /// starts at a consonant), replaying it cannot re-trigger a V-C-V split, so
    /// `key_log` is not truncated here. Cancels/expansions are re-applied exactly
    /// as in the forward path, converging `raw_chars`, `prev_rendered`,
    /// `last_valid_*` and the core `buf`/`raw`/`raw_len`/`out_buf` to a single
    /// consistent state — eliminating the "lossy replay" ghost-character bug.
    pub(crate) fn rebuild_composing(&mut self, log: &[char]) {
        // Reset the core engine to a clean slate (mirrors `clear()` minus diff).
        self.buf.clear();
        self.raw_len = 0;
        self.out_buf.clear();
        self.committed.clear();
        self.syl_structure.clear();

        // Reset diff composing state only.
        self.diff.raw_chars.clear();
        self.diff.prev_rendered.clear();
        self.diff.prev_inner_render.clear();
        self.diff.last_valid_raw_len = 0;
        self.diff.last_valid_coda_start = 0;
        self.diff.last_valid_out.clear();

        // Replay the surviving keystrokes through the core pipeline. This does
        // not touch `key_log` (the wrapper owns appends; V-C-V/full-buffer cannot
        // fire for a single syllable), so the log stays intact for the next
        // backspace.
        for &c in log {
            let _ = self.feed_diff_core(c);
        }
    }
}

// Static helper methods on UltraFastViEngine used by the diff module.
impl UltraFastViEngine {
    /// Compute minimal diff from `prev` → `new`, writing suffix into `out`.
    ///
    /// Returns `(backspaces, suffix_len)`. The caller sends `backspaces`
    /// delete keys then types the `suffix` chars to transform `prev` → `new`.
    pub(crate) fn diff_into(prev: &str, new: &str, out: &mut OutBuffer) -> (usize, usize) {
        // Single-pass: count common prefix and track prev char count
        // simultaneously, avoiding the double `.chars().count()` iteration.
        let mut common = 0usize;
        let mut prev_count = 0usize;
        let mut prev_iter = prev.chars();
        let mut new_iter = new.chars();
        loop {
            let p = prev_iter.next();
            if p.is_some() {
                prev_count += 1;
            }
            match (p, new_iter.next()) {
                (Some(a), Some(b)) if a == b => common += 1,
                _ => break,
            }
        }
        // Count remaining prev chars (after the common prefix)
        for _ in prev_iter {
            prev_count += 1;
        }
        let backspaces = prev_count - common;
        out.clear();
        for c in new.chars().skip(common) {
            let _ = out.push(c);
        }
        (backspaces, out.len())
    }

    /// Returns true for characters that end the current composing word.
    ///
    /// Any ASCII non-alphanumeric character is a word boundary — this covers
    /// `/`, `\`, `-`, `_`, `@`, `#`, etc. that users type mid-sentence
    /// (e.g. URLs, paths, code). Without this, a leading `/` would be pushed
    /// into the buffer as a literal consonant, corrupting `is_legal_onset`
    /// and silently disabling tone/modifier application for the rest of the
    /// word (see bug #11: `/duowcs` produced `/duowcs` instead of `/được`).
    ///
    /// Digits are NOT boundaries because VNI uses `0-9` as tone/modifier keys.
    /// Non-ASCII characters (incl. precomposed Vietnamese) are NOT boundaries
    /// here — they are decomposed by `feed()` and flow through the normal path.
    /// Unicode whitespace is still a boundary via `is_whitespace()`.
    #[inline]
    pub(crate) fn is_word_boundary(ch: char) -> bool {
        ch.is_whitespace() || (ch.is_ascii() && !ch.is_ascii_alphanumeric())
    }

    /// Returns true if the composed output equals the raw input (no Vietnamese transforms).
    #[inline]
    pub(crate) fn is_raw_passthrough_slice(raw: &[char], composed: &str) -> bool {
        if raw.is_empty() {
            return true;
        }
        let mut ci = composed.chars();
        for &r in raw {
            match ci.next() {
                Some(c) if c == r => {}
                _ => return false,
            }
        }
        ci.next().is_none()
    }

    /// Find the V-C-V split point: index in raw_chars where the second syllable starts.
    pub(crate) fn find_split_point(raw: &[char]) -> usize {
        let n = raw.len();
        if n == 0 {
            return 0;
        }
        let new_vowel_pos = n - 1;
        let mut last_old_vowel = 0usize;
        let mut found_old_vowel = false;
        for i in (0..new_vowel_pos).rev() {
            if Self::is_ascii_vowel(raw[i] as u8) {
                last_old_vowel = i;
                found_old_vowel = true;
                break;
            }
        }
        if !found_old_vowel {
            return 0;
        }
        if last_old_vowel < new_vowel_pos {
            let first_cons_after_vowel = (last_old_vowel + 1..new_vowel_pos)
                .find(|&i| !Self::is_ascii_vowel(raw[i] as u8))
                .unwrap_or(new_vowel_pos);
            return first_cons_after_vowel;
        }
        0
    }

    /// Re-render a slice of chars through the engine and return rendered output.
    ///
    /// On `std`: uses a thread-local scratch engine to avoid allocating a
    /// fresh `UltraFastViEngine` on every V-C-V split.
    /// On `no_std`: creates a new engine each time (no `thread_local!`).
    pub(crate) fn rerender_chars(raw: &[char], mode: &'static Mode) -> OutBuffer {
        #[cfg(feature = "std")]
        {
            thread_local! {
                static SCRATCH: std::cell::RefCell<UltraFastViEngine> =
                    std::cell::RefCell::new(UltraFastViEngine::new());
            }
            SCRATCH.with(|s| {
                let mut eng = s.borrow_mut();
                eng.clear();
                eng.mode = mode;
                for &c in raw {
                    eng.feed(c);
                }
                eng.out_buf.clone()
            })
        }
        #[cfg(not(feature = "std"))]
        {
            let mut eng = UltraFastViEngine::new();
            eng.mode = mode;
            for &c in raw {
                eng.feed(c);
            }
            eng.out_buf.clone()
        }
    }

    #[inline]
    pub(crate) fn is_ascii_vowel(b: u8) -> bool {
        matches!(b, b'a' | b'e' | b'i' | b'o' | b'u' | b'y')
    }

    #[inline]
    pub(crate) fn is_tone_key_in_mode(ch: char, mode: &Mode) -> bool {
        let b = ch as u8;
        mode.classify[b as usize] & IS_TONE_KEY != 0
    }

    #[inline]
    pub(crate) fn is_single_consonant_appended_slice(
        raw: &[char],
        last_valid_raw_len: usize,
    ) -> bool {
        if raw.len() != last_valid_raw_len + 1 {
            return false;
        }
        let ch = raw[last_valid_raw_len];
        !Self::is_ascii_vowel(ch as u8)
    }

    /// Find the raw index where the coda starts (index past the last vowel).
    /// If there is no vowel, the whole slice is treated as onset/coda.
    #[inline]
    pub(crate) fn raw_coda_start(raw: &[char]) -> usize {
        let mut last_vowel = None;
        for (i, &c) in raw.iter().enumerate() {
            if Self::is_ascii_vowel(c as u8) {
                last_vowel = Some(i);
            }
        }
        last_vowel.map(|i| i + 1).unwrap_or(0)
    }
}
