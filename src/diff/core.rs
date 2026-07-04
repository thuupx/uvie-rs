//! Core feed pipeline and backspace rebuild helpers for diff mode.
//!
//! `feed_diff_core` is the forward pipeline shared by the public `feed_diff`
//! (which appends to `key_log` first) and by `rebuild_composing` (which does
//! not). Keeping the append out of the core lets backspace replay keystrokes
//! without double-appending to the log.

use crate::buffers::{CharVec, new_out_buffer};
use crate::composing::Composable;
use crate::engine::UltraFastViEngine;

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
            let _ = self
                .diff
                .scratch_display
                .push_str(&self.diff.last_valid_out);
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
