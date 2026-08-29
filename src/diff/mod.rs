//! Diff-based input API and V-C-V syllable splitting.
//!
//! The diff engine wraps the core composing engine and computes minimal
//! (backspace_count, suffix_to_type) instructions for each keystroke.

mod core;
mod state;
mod utils;

pub use state::{ComposingSnapshot, DiffState};

use crate::engine::UltraFastViEngine;

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

        // Append to key_log and word_raw, then run the core pipeline.
        let _ = self.diff.key_log.try_push(ch);
        let _ = self.diff.word_raw.try_push(ch);
        // Only save pre-keystroke screen state if the dictionary override
        // could possibly fire (word_raw >= 4 chars, the minimum dict word).
        // This avoids 256 bytes of clones on the common path.
        let dict_eligible = self.diff.word_raw.len() >= 4;
        let committed_before = if dict_eligible {
            Some(self.diff.diff_committed.clone())
        } else {
            None
        };
        let prev_before = if dict_eligible {
            Some(self.diff.prev_rendered.clone())
        } else {
            None
        };
        let (bs, _suffix) = self.feed_diff_core(ch);

        // English dictionary override (per-keystroke): if the full word
        // typed so far matches a known English word, show the raw English
        // word instead of the garbled Vietnamese transform. This makes the
        // override visible WHILE typing, not just at the word boundary.
        //
        // The Vietnamese engine state (buf, raw_chars, etc.) is still
        // maintained in parallel — if the user continues typing past the
        // dictionary word (e.g. "characters"), the override stops firing
        // and the Vietnamese transform is shown again.
        if dict_eligible
            && crate::tables::is_english_override(&self.diff.word_raw)
        {
            let committed_before = committed_before.unwrap();
            let prev_before = prev_before.unwrap();
            // Build full on-screen text BEFORE this keystroke (the baseline
            // for the diff). Use the pre-feed_diff_core snapshots to avoid
            // double-counting V-C-V committed text.
            let mut full_before = crate::buffers::new_out_buffer();
            let _ = full_before.push_str(&committed_before);
            let _ = full_before.push_str(&prev_before);

            // Build target: full raw English word (preserve original case).
            let mut target = crate::buffers::new_out_buffer();
            for &c in self.diff.word_raw.iter() {
                let _ = target.push(c);
            }

            // Recompute diff from full_before → raw word.
            let (bs2, _) =
                Self::diff_into(&full_before, &target, &mut self.diff.diff_suffix);

            // Commit the English word to diff_committed and clear composing
            // state. This is CRITICAL: if we leave raw_chars/buf intact, the
            // next keystroke's V-C-V split will re-render the committed
            // portion from raw_chars, producing Vietnamese transforms (e.g.
            // "good" → "gô") and causing ghost characters.
            //
            // By committing the word and clearing composing state, the next
            // keystroke starts a fresh syllable. word_raw is preserved for
            // future dict checks (e.g. "goodness" matches later).
            self.diff.diff_committed.clear();
            let _ = self.diff.diff_committed.push_str(&target);
            self.diff.prev_rendered.clear();
            // Clear all composing state so feed_diff_core starts fresh.
            self.diff.raw_chars.clear();
            self.diff.key_log.clear();
            self.diff.prev_inner_render.clear();
            self.diff.last_valid_out.clear();
            self.diff.last_valid_raw_len = 0;
            self.diff.last_valid_coda_start = 0;
            // Clear core engine state (matches reset_diff minus diff.clear()).
            self.buf.clear();
            self.raw_len = 0;
            self.out_buf.clear();
            self.committed.clear();
            self.syl_structure.clear();
            // Clear snapshots — composing state is empty, no replay needed.
            for s in &mut self.diff.snapshots {
                *s = None;
            }
            self.diff.snapshot_count = 0;
            return (bs2, &self.diff.diff_suffix);
        }

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
        // Override state: prev_rendered is the raw English word (not the
        // Vietnamese transform). The engine's internal state (key_log,
        // raw_chars) is inconsistent with prev_rendered due to V-C-V split,
        // so replay would produce garbage. Instead, just pop one char from
        // prev_rendered and word_raw.
        //
        // Fast check: if prev_rendered contains any non-ASCII chars, it's
        // Vietnamese (not override state). This avoids the dict lookup and
        // String allocation on the common backspace path.
        if self.diff.word_raw.len() >= 4
            && self.diff.prev_rendered.is_ascii()
            && crate::tables::is_english_override(&self.diff.word_raw)
        {
            // Compare prev_rendered to word_raw (case-sensitive, since both
            // now preserve original case). Both are ASCII (is_ascii check).
            let prev_bytes = self.diff.prev_rendered.as_bytes();
            let matches = prev_bytes.len() == self.diff.word_raw.len()
                && prev_bytes.iter().zip(self.diff.word_raw.iter())
                    .all(|(b, c)| *b == *c as u8);
            if matches {
                self.diff.prev_rendered.pop();
                self.diff.word_raw.pop();
                if !self.diff.key_log.is_empty() {
                    self.diff.key_log.pop();
                }
                self.diff.diff_suffix.clear();
                return (1, &self.diff.diff_suffix);
            }
        }

        // No composing keystrokes left → fall back to popping auto-committed text
        // (V-C-V split output or English dict override) one rendered char at a time.
        if self.diff.key_log.is_empty() {
            if !self.diff.diff_committed.is_empty() {
                self.diff.diff_committed.pop();
                // Also pop word_raw to keep dict override tracking in sync.
                // Without this, backspace+retype would leave stale chars in
                // word_raw and the dict check would fail.
                self.diff.word_raw.pop();
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
            // word_raw is not snapshotted — just pop the last char, matching
            // key_log's pop in the replay path below.
            self.diff.word_raw.pop();
            self.diff.prev_rendered = snap.prev_rendered.clone();
            self.diff.prev_inner_render = snap.prev_inner_render;
            self.diff.last_valid_raw_len = snap.last_valid_raw_len;
            self.diff.last_valid_coda_start = snap.last_valid_coda_start;
            self.diff.last_valid_out = snap.last_valid_out;

            // Diff old vs new composing text.
            let (bs, _) =
                Self::diff_into(&prev, &self.diff.prev_rendered, &mut self.diff.diff_suffix);
            return (bs, &self.diff.diff_suffix);
        }

        // Fallback: O(n) replay path (used when snapshot stack is empty,
        // e.g. after V-C-V split or when snapshots were exhausted).
        let prev = self.diff.prev_rendered.clone();
        self.diff.key_log.pop();
        self.diff.word_raw.pop();
        let log: crate::buffers::CharVec<24> = self.diff.key_log.iter().copied().collect();
        self.rebuild_composing(&log);
        let new = self.diff.prev_rendered.clone();
        let (bs, _) = Self::diff_into(&prev, &new, &mut self.diff.diff_suffix);
        (bs, &self.diff.diff_suffix)
    }

    fn commit_diff(&mut self) -> (usize, &str) {
        // English dictionary override: if the full word matches a known
        // English word, replace the Vietnamese transform with the raw English
        // word. The diff engine computes the backspaces needed to transform
        // what's on screen (diff_committed + prev_rendered) into the raw word.
        if !self.diff.word_raw.is_empty()
            && crate::tables::is_english_override(&self.diff.word_raw)
        {
            // Build full on-screen text (Vietnamese) in a stack buffer.
            let mut full_screen = crate::buffers::new_out_buffer();
            let _ = full_screen.push_str(&self.diff.diff_committed);
            let _ = full_screen.push_str(&self.diff.prev_rendered);

            // Build target text (raw English word, preserve case) in a stack buffer.
            let mut target = crate::buffers::new_out_buffer();
            for &c in self.diff.word_raw.iter() {
                let _ = target.push(c);
            }

            // Clear all state.
            self.buf.clear();
            self.raw_len = 0;
            self.out_buf.clear();
            self.committed.clear();
            self.syl_structure.clear();
            self.diff.clear();
            for s in &mut self.diff.snapshots {
                *s = None;
            }
            self.diff.snapshot_count = 0;

            // Diff from full_screen → target, writing suffix into diff_suffix.
            let (bs, _) = Self::diff_into(&full_screen, &target, &mut self.diff.diff_suffix);
            return (bs, &self.diff.diff_suffix);
        }

        self.buf.clear();
        self.raw_len = 0;
        self.out_buf.clear();
        self.committed.clear();
        self.syl_structure.clear();
        self.diff.raw_chars.clear();
        self.diff.key_log.clear();
        self.diff.word_raw.clear();
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
        // Clear the snapshot stack — stale snapshots from the committed word
        // must not survive, otherwise a backspace after commit would restore
        // state from the previous word, corrupting the engine.
        for s in &mut self.diff.snapshots {
            *s = None;
        }
        self.diff.snapshot_count = 0;
        (0, &self.diff.diff_suffix)
    }

    fn reset_diff(&mut self) {
        // Full reset: clear both the core engine state AND the diff state.
        // Must be consistent with clear() + diff.clear() to avoid stale
        // committed text, syl_structure, or cached partition surviving.
        self.clear();
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
