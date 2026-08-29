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
            let (bs, _) =
                Self::diff_into(&prev, &self.diff.prev_rendered, &mut self.diff.diff_suffix);
            return (bs, &self.diff.diff_suffix);
        }

        // Fallback: O(n) replay path (used when snapshot stack is empty,
        // e.g. after V-C-V split or when snapshots were exhausted).
        let prev = self.diff.prev_rendered.clone();
        self.diff.key_log.pop();
        let log: crate::buffers::CharVec<24> = self.diff.key_log.iter().copied().collect();
        self.rebuild_composing(&log);
        let new = self.diff.prev_rendered.clone();
        let (bs, _) = Self::diff_into(&prev, &new, &mut self.diff.diff_suffix);
        (bs, &self.diff.diff_suffix)
    }

    fn commit_diff(&mut self) -> (usize, &str) {
        self.buf.clear();
        self.raw_len = 0;
        self.out_buf.clear();
        self.committed.clear();
        self.syl_structure.clear();
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
