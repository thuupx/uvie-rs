/// Fixed-capacity char buffer (replaces arrayvec dependency).
#[derive(Clone)]
pub struct CharVec<const N: usize> {
    data: [char; N],
    len: usize,
}

impl<const N: usize> Default for CharVec<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> CharVec<N> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            data: ['\0'; N],
            len: 0,
        }
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
        self.len >= N
    }

    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    #[inline]
    pub fn try_push(&mut self, ch: char) -> bool {
        if self.len < N {
            self.data[self.len] = ch;
            self.len += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<char> {
        if self.len > 0 {
            self.len -= 1;
            Some(self.data[self.len])
        } else {
            None
        }
    }

    #[inline]
    pub fn truncate(&mut self, new_len: usize) {
        if new_len < self.len {
            self.len = new_len;
        }
    }

    #[inline]
    pub fn swap(&mut self, a: usize, b: usize) {
        self.data.swap(a, b);
    }

    #[inline]
    pub fn as_slice(&self) -> &[char] {
        &self.data[..self.len]
    }

    #[inline]
    pub fn iter(&self) -> core::slice::Iter<'_, char> {
        self.as_slice().iter()
    }
}

impl<const N: usize> core::ops::Deref for CharVec<N> {
    type Target = [char];
    #[inline]
    fn deref(&self) -> &[char] {
        self.as_slice()
    }
}

impl<const N: usize> core::iter::FromIterator<char> for CharVec<N> {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        let mut v = Self::new();
        for ch in iter {
            if !v.try_push(ch) {
                break;
            }
        }
        v
    }
}

// ---------------------------------------------------------------------------
// Stack-allocated UTF-8 string buffer (replaces String for OutBuffer)
// ---------------------------------------------------------------------------

/// Fixed-capacity UTF-8 string backed by a stack `[u8; N]` array.
///
/// This is the `std`-build equivalent of `heapless::String<N>`. It stores
/// UTF-8 bytes directly so `Deref<Target = str>` is zero-cost — no
/// `[char]`-to-`str` conversion or heap allocation needed.
///
/// `clone()` copies a fixed `[u8; N]` array + `usize` len — entirely on
/// stack, no heap. For N=128 that's 136 bytes per clone vs `String::clone`
/// which always allocates.
///
/// # Safety
///
/// The buffer only accepts valid UTF-8 input (via `push(char)` and
/// `push_str(&str)`), so `str::from_utf8_unchecked` in `Deref` is sound.
#[derive(Clone)]
pub struct StackStr<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> Default for StackStr<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> StackStr<N> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
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
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Push a single char as UTF-8 bytes. Returns `false` if the buffer
    /// is full (no partial write).
    #[inline]
    pub fn push(&mut self, ch: char) -> bool {
        let mut buf = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buf);
        let encoded_len = encoded.len();
        if self.len + encoded_len > N {
            return false;
        }
        self.bytes[self.len..self.len + encoded_len]
            .copy_from_slice(&buf[..encoded_len]);
        self.len += encoded_len;
        true
    }

    /// Push a `&str` as UTF-8 bytes. Returns `false` if the buffer is full.
    #[inline]
    pub fn push_str(&mut self, s: &str) -> bool {
        let s_bytes = s.as_bytes();
        if self.len + s_bytes.len() > N {
            return false;
        }
        self.bytes[self.len..self.len + s_bytes.len()].copy_from_slice(s_bytes);
        self.len += s_bytes.len();
        true
    }

    /// Remove and return the last char. Returns `None` if empty.
    #[inline]
    pub fn pop(&mut self) -> Option<char> {
        if self.len == 0 {
            return None;
        }
        // Find the start of the last UTF-8 char by scanning back 1-4 bytes.
        let last = self.bytes[self.len - 1];
        let char_start = if last < 0x80 {
            self.len - 1
        } else {
            // Multi-byte: find the start byte (0b11xx_xxxx)
            let mut start = self.len - 1;
            while start > 0 && self.bytes[start] & 0xC0 == 0x80 {
                start -= 1;
            }
            start
        };
        let s = core::str::from_utf8(&self.bytes[char_start..self.len]).ok()?;
        let ch = s.chars().next()?;
        self.len = char_start;
        Some(ch)
    }

    /// Returns the bytes that have been written.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl<const N: usize> core::ops::Deref for StackStr<N> {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        // SAFETY: We only accept valid UTF-8 via push(char) and push_str(&str).
        // Both inputs are guaranteed valid UTF-8, so the byte buffer is too.
        unsafe { core::str::from_utf8_unchecked(&self.bytes[..self.len]) }
    }
}

impl<const N: usize> core::fmt::Display for StackStr<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self)
    }
}

impl<const N: usize> core::fmt::Debug for StackStr<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}

impl<const N: usize> PartialEq for StackStr<N> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<const N: usize> Eq for StackStr<N> {}

impl<const N: usize> PartialEq<str> for StackStr<N> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        &**self == other
    }
}

// ---------------------------------------------------------------------------
// Type aliases — StackStr for both std and no_std (zero heap, zero deps)
// ---------------------------------------------------------------------------

pub type RawBuffer = StackStr<64>;
pub type OutBuffer = StackStr<128>;

#[inline(always)]
pub fn new_raw_buffer() -> RawBuffer {
    StackStr::new()
}

#[inline(always)]
pub fn new_out_buffer() -> OutBuffer {
    StackStr::new()
}
