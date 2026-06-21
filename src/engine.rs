use crate::buffers::{OutBuffer, new_out_buffer};
use crate::composing::Composable;
use crate::diff::DiffState;
use crate::modes::{InputMethod, Mode, mode_for};
use crate::syllable::{SylBuf, SylStructure};

// ---------------------------------------------------------------------------
// Public engine struct
// ---------------------------------------------------------------------------

pub struct UltraFastViEngine {
    /// Per-char buffer for the current composing word.
    pub(crate) buf: SylBuf,
    /// Raw keystroke snapshot. `raw[i]` == lowercased byte of the i-th key.
    pub(crate) raw: [u8; 24],
    /// Uppercase flag for each raw keystroke.
    pub(crate) raw_caps: [bool; 24],
    pub(crate) raw_len: usize,
    /// Rendered output of the current composing word.
    pub(crate) out_buf: OutBuffer,
    /// Accumulated committed text (prior complete words + boundary chars).
    pub(crate) committed: OutBuffer,
    /// Input method - determines classifier and tone tables.
    pub(crate) input_method: InputMethod,
    pub(crate) mode: &'static Mode,
    /// Engine configuration flags.
    pub(crate) enable_quick_start: bool,
    pub(crate) enable_quick_telex: bool,
    pub(crate) enable_modern_orthography: bool,
    pub(crate) enable_relaxed_coda: bool,

    /// Incrementally maintained syllable structure (onset/nucleus/coda slots).
    pub(crate) syl_structure: SylStructure,

    /// Diff-mode state (V-C-V splitting, screen diffing).
    pub(crate) diff: DiffState,
}

impl UltraFastViEngine {
    pub fn new() -> Self {
        let input_method = InputMethod::Telex;
        Self {
            buf: SylBuf::new(),
            raw: [0u8; 24],
            raw_caps: [false; 24],
            raw_len: 0,
            out_buf: new_out_buffer(),
            committed: new_out_buffer(),
            input_method,
            mode: mode_for(input_method),
            enable_quick_start: false,
            enable_quick_telex: false,
            enable_modern_orthography: false,
            enable_relaxed_coda: false,
            syl_structure: SylStructure::new(),
            diff: DiffState::new(),
        }
    }

    // ------------------------------------------------------------------
    // Configuration accessors
    // ------------------------------------------------------------------

    pub fn set_quick_start(&mut self, enabled: bool) {
        self.enable_quick_start = enabled;
    }
    pub fn quick_start(&self) -> bool {
        self.enable_quick_start
    }

    pub fn set_quick_telex(&mut self, enabled: bool) {
        self.enable_quick_telex = enabled;
    }
    pub fn quick_telex(&self) -> bool {
        self.enable_quick_telex
    }

    pub fn set_modern_orthography(&mut self, enabled: bool) {
        self.enable_modern_orthography = enabled;
    }
    pub fn modern_orthography(&self) -> bool {
        self.enable_modern_orthography
    }

    pub fn set_relaxed_coda(&mut self, enabled: bool) {
        self.enable_relaxed_coda = enabled;
    }
    pub fn relaxed_coda(&self) -> bool {
        self.enable_relaxed_coda
    }

    pub fn set_input_method(&mut self, method: InputMethod) {
        self.input_method = method;
        self.mode = mode_for(method);
    }
    pub fn input_method(&self) -> InputMethod {
        self.input_method
    }

    // ------------------------------------------------------------------
    // State queries
    // ------------------------------------------------------------------

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty() && self.committed.is_empty()
    }

    pub fn is_composing(&self) -> bool {
        !self.buf.is_empty()
    }

    /// Current logical raw length (may differ from push count due to double-cancel).
    pub fn raw_len(&self) -> usize {
        self.raw_len
    }

    /// Length of the diff-mode raw character buffer.
    pub fn raw_chars_len(&self) -> usize {
        self.diff.raw_chars.len()
    }

    /// Copy the diff-mode raw characters into a `String` for debugging/tests.
    #[cfg(feature = "std")]
    pub fn raw_chars_string(&self) -> String {
        self.diff.raw_chars.iter().collect()
    }

    pub fn current_composing(&self) -> &str {
        &self.out_buf
    }

    /// Returns the classify flags for a raw byte in the current input mode.
    #[inline]
    pub fn mode_classify(&self, b: u8) -> u8 {
        self.mode.classify[b as usize]
    }

    pub fn committed_text(&self) -> &str {
        &self.committed
    }

    #[cfg(feature = "std")]
    pub fn current_output(&self) -> String {
        let mut s = String::with_capacity(self.committed.len() + self.out_buf.len());
        s.push_str(&self.committed);
        s.push_str(&self.out_buf);
        s
    }

    /// Returns the current syllable structure (onset/nucleus/coda slots).
    #[inline]
    pub fn syl_structure(&self) -> &SylStructure {
        &self.syl_structure
    }

    // ------------------------------------------------------------------
    // Lifecycle
    // ------------------------------------------------------------------

    pub fn clear(&mut self) {
        self.buf.clear();
        self.raw_len = 0;
        self.out_buf.clear();
        self.committed.clear();
        self.syl_structure.clear();
    }

    /// Finalise the composing word into `committed` and reset composing state.
    pub fn commit(&mut self) {
        self.render_out_buf();
        let _ = self.committed.push_str(&self.out_buf);
        self.buf.clear();
        self.raw_len = 0;
        self.out_buf.clear();
    }

    /// Delete the last typed key (backspace). Returns the new composing text.
    pub fn backspace(&mut self) -> &str {
        if self.raw_len > 0 {
            self.raw_len -= 1;
            let target_len = self.raw_len;
            self.buf.clear();
            self.raw_len = 0;
            for i in 0..target_len {
                let b = self.raw[i];
                let caps = self.raw_caps[i];
                self.raw[self.raw_len] = b;
                self.raw_caps[self.raw_len] = caps;
                self.raw_len += 1;
                self.process_key(b, caps);
            }
            self.render_out_buf();
            return &self.out_buf;
        }
        if !self.committed.is_empty() {
            self.committed.pop();
        }
        self.out_buf.clear();
        &self.out_buf
    }

    // ------------------------------------------------------------------
    // Core feed method
    // ------------------------------------------------------------------

    /// Feed one character. Returns the current composing text (not including
    /// committed text). Whitespace commits the current word.
    pub fn feed(&mut self, key: char) -> &str {
        if key.is_whitespace() {
            self.render_out_buf();
            let _ = self.committed.push_str(&self.out_buf);
            self.buf.clear();
            self.raw_len = 0;
            self.out_buf.clear();
            let _ = self.committed.push(key);
            return &self.out_buf;
        }

        let lower = key.to_ascii_lowercase();
        let caps = key != lower;

        if self.enable_quick_start {
            match lower {
                'j' => {
                    self.push_raw_key(b'g', false);
                    self.push_raw_key(b'i', false);
                }
                'f' => {
                    self.push_raw_key(b'p', false);
                    self.push_raw_key(b'h', false);
                }
                'w' => {
                    self.push_raw_key(b'q', false);
                    self.push_raw_key(b'u', false);
                }
                _ => {
                    self.push_raw_key(lower as u8, caps);
                }
            }
        } else if self.enable_quick_telex && self.raw_len > 0 {
            let prev = self.raw[self.raw_len - 1];
            let expansion: Option<[u8; 2]> = match lower {
                'c' => Some([b'c', b'h']),
                'g' => Some([b'g', b'i']),
                'k' => Some([b'k', b'h']),
                'n' => Some([b'n', b'g']),
                'q' => Some([b'q', b'u']),
                'p' => Some([b'p', b'h']),
                't' => Some([b't', b'h']),
                _ => None,
            };
            if let Some(pair) = expansion {
                if prev == lower as u8 {
                    self.raw_len -= 1;
                    self.buf.pop();
                    self.push_raw_key(pair[0], false);
                    self.push_raw_key(pair[1], false);
                } else {
                    self.push_raw_key(lower as u8, caps);
                }
            } else {
                self.push_raw_key(lower as u8, caps);
            }
        } else {
            self.push_raw_key(lower as u8, caps);
        }

        self.render_out_buf();
        &self.out_buf
    }
}

impl Default for UltraFastViEngine {
    fn default() -> Self {
        Self::new()
    }
}
