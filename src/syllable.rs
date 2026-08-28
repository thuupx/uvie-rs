//! Per-keystroke state for the incremental Vietnamese engine.
//!
//! Each key typed into the current word gets one `Syl` entry. The `Syl` stores
//! the raw key (`base`), the resolved display character (`out`), the applied
//! tone (`tone`), and a bitfield of modifier flags (`flags`).
//!
//! # Design rationale
//!
//! Keeping per-char state lets the engine do transforms by flipping bits on
//! the right element and re-rendering only the affected suffix.
//!
//! Validation always operates on the raw `base` sequence, never on resolved
//! `out` chars, which is what makes English passthrough automatic.

use crate::tone::map_vowel_with_tone;

// ---------------------------------------------------------------------------
// Flag constants
// ---------------------------------------------------------------------------

/// `â ê ô` - double-vowel circumflex (telex: aa/ee/oo, VNI: 6).
pub const F_CIRCUMFLEX: u8 = 1 << 0;

/// `ă ơ ư` - breve/horn modifier (telex: aw/ow/uw, VNI: 7/8).
pub const F_HORN: u8 = 1 << 1;

/// The physical key was uppercase (shift held).
pub const F_CAPS: u8 = 1 << 2;

/// This entry must be emitted verbatim - no further transforms allowed.
/// Set when a triple-cancel fires or the word becomes an English passthrough.
pub const F_LITERAL: u8 = 1 << 3;

/// `tone` field is meaningful. Distinguishes "tone explicitly cleared (z key,
/// tone 0 + F_TONE_SET)" from "no tone key typed yet (tone 0, no F_TONE_SET)".
pub const F_TONE_SET: u8 = 1 << 4;

// ---------------------------------------------------------------------------
// Syl struct
// ---------------------------------------------------------------------------

/// One entry per physical key kept in the current composing word.
///
/// Tone numbering mirrors `TONE_TELEX` / `TONE_VNI` in `modes.rs`:
/// `1=sắc(s), 2=huyền(f), 3=hỏi(r), 4=ngã(x), 5=nặng(j), 0=bằng/remove(z)`.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Syl {
    /// Raw ASCII key as typed, lowercased: `b'a'`, `b'e'`, `b'd'`, `b's'`, …
    pub base: u8,

    /// Current resolved display character: `'a'`, `'â'`, `'ế'`, `'đ'`, …
    /// Recomputed whenever `flags` or `tone` change via `Syl::render`.
    pub out: char,

    /// Tone index 0–5 (see numbering above). Only meaningful when `F_TONE_SET`
    /// is set in `flags`.
    pub tone: u8,

    /// Bitfield: see `F_*` constants above.
    pub flags: u8,
}

impl Syl {
    /// Construct a new literal entry (no modifier, no tone).
    ///
    /// **Note**: `literal` also sets no F_LITERAL flag - the name refers to the
    /// fact that no Vietnamese transform has been applied yet. F_LITERAL is set
    /// by the engine when the word is forced into raw passthrough (triple-cancel).
    #[inline]
    pub fn literal(base: u8, caps: bool) -> Self {
        let out = if caps {
            (base as char).to_ascii_uppercase()
        } else {
            base as char
        };
        Self {
            base,
            out,
            tone: 0,
            flags: if caps { F_CAPS } else { 0 },
        }
    }

    /// Construct a plain consonant entry. Alias for `literal`, exists for
    /// semantic clarity at call sites (e.g. double-tone cancel coda).
    #[inline]
    pub fn consonant(base: u8, caps: bool) -> Self {
        Self::literal(base, caps)
    }

    /// Re-derive `out` from `base + flags + tone` and return `self` with the
    /// updated field. The caller stores the returned value.
    ///
    /// `tone_target` is the index within the nucleus that should receive the
    /// tone diacritic, as provided by `tables::nucleus_tone_target`. Pass
    /// `is_tone_carrier = true` on the vowel that should carry the mark.
    #[inline]
    pub fn with_tone(mut self, tone: u8) -> Self {
        self.tone = tone;
        self.flags |= F_TONE_SET;
        // `out` will be recomputed during full render; mark it dirty here.
        // The engine calls `recompute_out` on the tone carrier explicitly.
        self
    }

    /// Apply circumflex modifier (aa→â, ee→ê, oo→ô).
    #[inline]
    pub fn with_circumflex(mut self) -> Self {
        self.flags |= F_CIRCUMFLEX;
        self.flags &= !F_HORN;
        self.out = circumflex_base(self.base);
        if self.flags & F_CAPS != 0 {
            self.out = self.out.to_uppercase().next().unwrap_or(self.out);
        }
        // Re-apply tone if already set.
        if self.flags & F_TONE_SET != 0 {
            self.out = map_vowel_with_tone(self.out, self.tone);
        }
        self
    }

    /// Apply horn/breve modifier (aw→ă, ow→ơ, uw→ư).
    #[inline]
    pub fn with_horn(mut self) -> Self {
        self.flags |= F_HORN;
        self.flags &= !F_CIRCUMFLEX;
        self.out = horn_base(self.base);
        if self.flags & F_CAPS != 0 {
            self.out = self.out.to_uppercase().next().unwrap_or(self.out);
        }
        if self.flags & F_TONE_SET != 0 {
            self.out = map_vowel_with_tone(self.out, self.tone);
        }
        self
    }

    /// Remove modifier flags and reset `out` to plain base char.
    #[inline]
    pub fn clear_modifier(mut self) -> Self {
        self.flags &= !(F_CIRCUMFLEX | F_HORN);
        self.out = if self.flags & F_CAPS != 0 {
            (self.base as char).to_ascii_uppercase()
        } else {
            self.base as char
        };
        if self.flags & F_TONE_SET != 0 {
            self.out = map_vowel_with_tone(self.out, self.tone);
        }
        self
    }

    /// Recompute `out` from current flags and tone without changing anything
    /// else. Called after tone is set/moved to this position.
    #[inline]
    pub fn recompute_out(&mut self) {
        let base_char = if self.flags & F_CIRCUMFLEX != 0 {
            circumflex_base(self.base)
        } else if self.flags & F_HORN != 0 {
            horn_base(self.base)
        } else {
            self.base as char
        };
        let c = if self.flags & F_TONE_SET != 0 {
            map_vowel_with_tone(base_char, self.tone)
        } else {
            base_char
        };
        self.out = if self.flags & F_CAPS != 0 {
            c.to_uppercase().next().unwrap_or(c)
        } else {
            c
        };
    }

    /// Returns `true` if this entry is a vowel (base is a-e-i-o-u-y).
    #[inline]
    pub fn is_vowel(&self) -> bool {
        matches!(self.base, b'a' | b'e' | b'i' | b'o' | b'u' | b'y')
    }

    /// Returns `true` if this entry carries the literal flag.
    #[inline]
    pub fn is_literal(&self) -> bool {
        self.flags & F_LITERAL != 0
    }

    /// Returns the resolved `out` character.
    #[inline]
    pub fn render(&self) -> char {
        self.out
    }

    /// Returns the modifier-resolved base character WITHOUT tone.
    ///
    /// Used for nucleus validation: the nucleus table contains vowels after
    /// circumflex/horn transform but before tone diacritics.
    /// E.g. `á` (a + sắc) → `a`, `ế` (ê + sắc) → `ê`, `â` (circumflex) → `â`.
    ///
    /// Special case: `d + F_HORN` = `đ` (the horn flag is repurposed for đ).
    #[inline]
    pub fn base_no_tone(&self) -> char {
        if self.base == b'd' && self.flags & F_HORN != 0 {
            // đ (the 'd' with horn-flag is the đ modifier result)
            return 'đ';
        }
        if self.flags & F_CIRCUMFLEX != 0 {
            circumflex_base(self.base)
        } else if self.flags & F_HORN != 0 {
            horn_base(self.base)
        } else {
            self.base as char
        }
    }
}

// ---------------------------------------------------------------------------
// Modifier resolution helpers
// ---------------------------------------------------------------------------

/// Base character after applying circumflex (`aa→â`, `ee→ê`, `oo→ô`).
#[inline]
fn circumflex_base(base: u8) -> char {
    match base {
        b'a' => 'â',
        b'e' => 'ê',
        b'o' => 'ô',
        _ => base as char,
    }
}

/// Base character after applying horn/breve (`aw→ă`, `ow→ơ`, `uw→ư`).
#[inline]
fn horn_base(base: u8) -> char {
    match base {
        b'a' => 'ă',
        b'o' => 'ơ',
        b'u' | b'w' => 'ư',
        _ => base as char,
    }
}

// ---------------------------------------------------------------------------
// Fixed-size word buffer (no heap)
// ---------------------------------------------------------------------------

/// Maximum keys tracked in a single composing word. Vietnamese syllables are
/// at most ~8 raw keys; 24 gives ample headroom for VNI digits and edge cases.
pub const MAX_WORD_LEN: usize = 24;

/// A fixed-capacity buffer of [`Syl`] entries for the current composing word.
///
/// Uses a plain array + length counter - zero heap, `no_std`-compatible.
/// Tracks a `version` counter that increments on every mutation, enabling
/// O(1) cache validity checks for `partition_syllable`.
#[derive(Clone)]
pub struct SylBuf {
    entries: [Syl; MAX_WORD_LEN],
    len: usize,
    version: u32,
}

impl SylBuf {
    #[inline]
    pub const fn new() -> Self {
        Self {
            entries: [Syl {
                base: 0,
                out: '\0',
                tone: 0,
                flags: 0,
            }; MAX_WORD_LEN],
            len: 0,
            version: 0,
        }
    }

    /// Current version — increments on every mutation.
    /// Used by `partition_syllable` cache to detect staleness.
    #[inline]
    pub fn version(&self) -> u32 {
        self.version
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.len >= MAX_WORD_LEN
    }

    /// Append a [`Syl`]. Silently drops the entry if the buffer is full.
    #[inline]
    pub fn push(&mut self, s: Syl) {
        if self.len < MAX_WORD_LEN {
            self.entries[self.len] = s;
            self.len += 1;
            self.version = self.version.wrapping_add(1);
        }
    }

    /// Remove and return the last entry, or `None` if empty.
    #[inline]
    pub fn pop(&mut self) -> Option<Syl> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        self.version = self.version.wrapping_add(1);
        Some(self.entries[self.len])
    }

    /// Remove the entry at index `idx`, shifting subsequent entries left.
    #[inline]
    pub fn remove(&mut self, idx: usize) {
        if idx >= self.len {
            return;
        }
        for i in idx..self.len - 1 {
            self.entries[i] = self.entries[i + 1];
        }
        self.len -= 1;
        self.version = self.version.wrapping_add(1);
    }

    /// Clear all entries.
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
        self.version = self.version.wrapping_add(1);
    }

    /// Immutable slice of live entries.
    #[inline]
    pub fn as_slice(&self) -> &[Syl] {
        &self.entries[..self.len]
    }

    /// Mutable slice of live entries.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [Syl] {
        self.version = self.version.wrapping_add(1);
        &mut self.entries[..self.len]
    }

    /// Get entry by index (panics on out-of-bounds in debug, UB in release via
    /// `get_unchecked` - callers are responsible for bounds checks).
    #[inline]
    pub fn get(&self, i: usize) -> &Syl {
        &self.entries[i]
    }

    #[inline]
    pub fn get_mut(&mut self, i: usize) -> &mut Syl {
        self.version = self.version.wrapping_add(1);
        &mut self.entries[i]
    }

    /// Replace the entry at index `i`.
    #[inline]
    pub fn set(&mut self, i: usize, s: Syl) {
        self.entries[i] = s;
        self.version = self.version.wrapping_add(1);
    }

    /// Swap entries at indices `a` and `b`.
    #[inline]
    pub fn swap(&mut self, a: usize, b: usize) {
        self.entries.swap(a, b);
        self.version = self.version.wrapping_add(1);
    }
}

impl Default for SylBuf {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Typed Syllable Slots - incremental structure tracking
// ---------------------------------------------------------------------------

/// Classification of onset consonant patterns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnsetKind {
    /// No onset yet.
    None,
    /// Single consonant: b, c, d, g, h, k, l, m, n, p, q, r, s, t, v, x.
    Single(u8),
    /// Two-consonant digraph: ch, gh, gi, kh, ng, nh, ph, qu, th, tr.
    Digraph(u8, u8),
    /// Three-consonant trigraph: ngh.
    Trigraph,
}

/// Classification of nucleus vowel patterns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NucleusKind {
    /// No vowel yet.
    None,
    /// Single vowel: a, ă, â, e, ê, i, o, ô, ơ, u, ư, y.
    Single,
    /// Diphthong: ai, ao, au, ay, âu, ây, eo, êu, ia, iê, iu, oa, oă, oe, oi,
    /// ôi, ơi, ua, uâ, uê, ui, uô, ươ, ưa, ưi, ưu, ya, yê.
    Diphthong,
    /// Triphthong: iêu, oai, oay, uây, uôi, ươi, uyê, uya.
    Triphthong,
}

/// Incrementally maintained syllable structure indices.
///
/// These track the same information as `partition_syllable()` but are updated
/// on each keystroke instead of recomputed from scratch. The engine uses
/// `debug_assert_eq!` to validate consistency with `partition_syllable()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SylStructure {
    /// Index past the last onset entry (= nucleus_start).
    pub onset_end: usize,
    /// Index past the last nucleus entry (= coda_start).
    pub nucleus_end: usize,
    /// Onset classification.
    pub onset_kind: OnsetKind,
    /// Nucleus classification.
    pub nucleus_kind: NucleusKind,
}

impl SylStructure {
    pub const fn new() -> Self {
        Self {
            onset_end: 0,
            nucleus_end: 0,
            onset_kind: OnsetKind::None,
            nucleus_kind: NucleusKind::None,
        }
    }

    /// Reset to empty state.
    #[inline]
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Nucleus start index (same as onset_end).
    #[inline]
    pub fn nucleus_start(&self) -> usize {
        self.onset_end
    }

    /// Coda start index (same as nucleus_end).
    #[inline]
    pub fn coda_start(&self) -> usize {
        self.nucleus_end
    }

    /// Returns the 4-tuple `(onset_end, nucleus_start, nucleus_end, coda_start)`
    /// matching the format of `partition_syllable()`.
    #[inline]
    pub fn as_tuple(&self) -> (usize, usize, usize, usize) {
        (
            self.onset_end,
            self.onset_end,
            self.nucleus_end,
            self.nucleus_end,
        )
    }
}

impl Default for SylStructure {
    fn default() -> Self {
        Self::new()
    }
}
