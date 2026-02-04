#![cfg_attr(not(feature = "std"), no_std)]

const IS_VOWEL: u8 = 1 << 0;
const IS_MODIFIER: u8 = 1 << 1;
const IS_TONE_KEY: u8 = 1 << 2; // Keys: s, f, r, x, j, z

#[cfg(feature = "heapless")]
type RawBuffer = heapless::String<32>;

#[cfg(feature = "heapless")]
type OutBuffer = heapless::String<128>;

#[cfg(not(feature = "heapless"))]
type RawBuffer = String;

#[cfg(not(feature = "heapless"))]
type OutBuffer = String;

#[cfg(all(not(feature = "std"), not(feature = "heapless")))]
compile_error!(
    "no_std build requires `heapless` feature (use --no-default-features --features heapless)"
);

#[cfg(test)]
mod tests;

pub struct UltraFastViEngine {
    raw_buffer: RawBuffer,
    out_buffer: OutBuffer,
}

impl UltraFastViEngine {
    pub fn new() -> Self {
        Self {
            raw_buffer: new_raw_buffer(),
            out_buffer: new_out_buffer(),
        }
    }

    pub fn feed(&mut self, key: char) -> &str {
        if key.is_whitespace() {
            self.render_str();
            self.raw_buffer.clear();
            let _ = self.out_buffer.push(key);
            return &self.out_buffer;
        }
        let _ = self.raw_buffer.push(key.to_ascii_lowercase());
        self.render_str()
    }

    fn render_str(&mut self) -> &str {
        if self.raw_buffer.is_empty() {
            self.out_buffer.clear();
            return &self.out_buffer;
        }

        let bytes_all = self.raw_buffer.as_bytes();
        let bytes = &bytes_all[..bytes_all.len().min(32)];
        let mut processed = [0u8; 32];
        let mut p_len = 0;
        let mut last_tone_char = 0u8;

        // 1. Filter & Capture Tone
        for (idx, &b) in bytes.iter().enumerate() {
            let attr = classify(b);
            let is_tone = (attr & IS_TONE_KEY) != 0;

            if is_tone {
                // Rule 1: First character is always treated as consonant/content
                if idx == 0 {
                    processed[p_len] = b;
                    p_len += 1;
                    continue;
                }

                // Rule 2: 'r' after 't' is 'tr'
                // Extended: 'r' after 'p', 'f', 'c', 'b', 'd', 'g', 'k' is also treated as consonant cluster
                // to support "pro", "free", "cry", "bra", "dry", "gra", "kra" (mostly for English/foreign preservation)
                if b == b'r' && idx > 0 {
                    let prev = bytes[idx - 1];
                    if matches!(prev, b't' | b'p' | b'f' | b'c' | b'b' | b'd' | b'g' | b'k') {
                        processed[p_len] = b;
                        p_len += 1;
                        continue;
                    }
                }

                last_tone_char = b;
            } else {
                processed[p_len] = b;
                p_len += 1;
            }
        }

        // 1.5. Pre-process Toggling (ddd -> d, aaa -> a)
        // We do this in-place on `processed` if possible, or use a second pass.
        // Given max len is small, a second pass is cheap.
        // Handle: aa->â, aaa->a. dd->đ, ddd->d.
        // Logic: specific chars allow toggling: a, e, o, d.
        // Note: We only reduce triplets. Pairs are handled by resolve_telex.
        let mut toggled = [0u8; 32];
        let mut t_len = 0;
        let mut i = 0;
        while i < p_len {
            let c = processed[i];
            // Check for triplet
            if i + 2 < p_len && processed[i + 1] == c && processed[i + 2] == c {
                // Found triplet, check if it's a togglable char
                match c {
                    b'a' | b'e' | b'o' | b'd' => {
                        toggled[t_len] = c;
                        t_len += 1;
                        i += 3; // Skip 3, write 1
                        continue;
                    }
                    _ => {}
                }
            }
            toggled[t_len] = c;
            t_len += 1;
            i += 1;
        }

        // 1.6. Retroactive 'w' bubbling (single-pass insertion)
        // If we see 'w', bubble it leftwards until it hits a char it can modify (a, o, u, d)
        {
            let mut bubbled = [0u8; 32];
            let mut b_len = 0usize;
            let mut last_target_pos: Option<usize> = None;

            for k in 0..t_len {
                let c = toggled[k];
                if c == b'w' {
                    if let Some(tp) = last_target_pos {
                        if b_len < 32 {
                            let mut i = b_len;
                            while i > tp + 1 {
                                bubbled[i] = bubbled[i - 1];
                                i -= 1;
                            }
                            bubbled[tp + 1] = b'w';
                            b_len += 1;
                        }
                    } else {
                        if b_len < 32 {
                            bubbled[b_len] = b'w';
                            b_len += 1;
                        }
                    }
                } else {
                    bubbled[b_len] = c;
                    b_len += 1;
                    if is_w_target(c) {
                        last_target_pos = Some(b_len - 1);
                    }
                }
            }

            toggled = bubbled;
            t_len = b_len;
        }

        // 2. Resolve Telex & Build Char Buffer
        // We use a char buffer to avoid intermediate String allocations
        let mut char_buf = ['\0'; 32];
        let mut c_len = 0;
        let mut vowel_mask = 0u16;
        let mut v_idx = 0;

        i = 0;
        while i < t_len {
            let curr = toggled[i];
            let next = if i + 1 < t_len {
                Some(toggled[i + 1])
            } else {
                None
            };

            let (mut c, consumed) = resolve_telex(curr, next);

            // Fix uow -> ươ
            // Logic: u + o + w -> ươ
            // EXCEPTION: if preceded by 'q', then it is qu + o + w -> qu + ơ
            if curr == b'u' && !consumed {
                if let Some(n) = next {
                    if n == b'o' {
                        if i + 2 < t_len && toggled[i + 2] == b'w' {
                            // Check previous char for 'q'
                            let is_qu = if i > 0 {
                                let prev = toggled[i - 1];
                                prev == b'q' || prev == b'Q'
                            } else {
                                false
                            };

                            if !is_qu {
                                c = 'ư';
                                // We essentially mapped u -> ư.
                                // The loop will proceed. next iter: o, w -> ơ.
                                // Result: ư, ơ.
                            }
                        }
                    }
                }
            }

            if is_vowel_unicode(c) {
                if v_idx < 16 {
                    vowel_mask |= 1 << v_idx;
                }
            }

            char_buf[c_len] = c;
            c_len += 1;
            v_idx += 1;

            i += if consumed { 2 } else { 1 };
        }

        // 3. Validation
        // Check invalid Vietnamese combinations (e.g. English words like "clear")
        {
            let valid = !self.is_invalid_vietnamese_chars(&char_buf[..c_len], vowel_mask);
            if !valid {
                // If invalid, fallback to raw buffer
                // But wait, raw buffer might be different from char_buf without tone.
                // Actually if we return raw_buffer here, we return the whole string including the tone key at the end/position.
                self.out_buffer.clear();
                let _ = self.out_buffer.push_str(&self.raw_buffer);
                return &self.out_buffer;
            }
        }

        // 4. Tone Placement
        if last_tone_char > 0 {
            let tone_id = map_tone(last_tone_char);
            // tone_id 0 means remove tone (z key)
            self.apply_tone_in_place(&mut char_buf[..c_len], vowel_mask, tone_id);
        }

        self.out_buffer.clear();
        for &c in &char_buf[..c_len] {
            let _ = self.out_buffer.push(c);
        }
        &self.out_buffer
    }

    fn is_invalid_vietnamese_chars(&self, chars: &[char], vowel_mask: u16) -> bool {
        if vowel_mask == 0 {
            // If no vowels, it's valid only if it's a single character (e.g. "đ", "b")
            // Longer strings without vowels are considered invalid (e.g. "dd" -> "đ" is len 1, but "fr" is len 2)
            return chars.len() > 1;
        }

        // Bitmask-based adjacency checks (still overall O(n) to build masks, but O(1) to query patterns).
        // We limit masks to the first 32 chars (engine buffers are capped at 32).
        let mut mask_o: u32 = 0;
        let mut mask_u: u32 = 0;
        let mut idx: u32 = 0;
        for &c in chars.iter() {
            if idx >= 32 {
                break;
            }
            // Only care about ASCII 'o'/'u' here.
            if c == 'o' {
                mask_o |= 1u32 << idx;
            } else if c == 'u' {
                mask_u |= 1u32 << idx;
            }
            idx += 1;
        }

        // "ou" check: any position i with 'o' and i+1 with 'u'
        if (mask_o & (mask_u >> 1)) != 0 {
            return true;
        }

        let first_vowel_pos = vowel_mask.trailing_zeros() as usize;

        // Check initial consonant cluster length
        if first_vowel_pos >= 3 {
            // "ngh" is valid (length 3)
            if first_vowel_pos == 3 {
                if chars.len() >= 3 && chars[0] == 'n' && chars[1] == 'g' && chars[2] == 'h' {
                    return false;
                }
            }
            return true;
        }

        // Check specific invalid clusters of length 2
        if first_vowel_pos == 2 {
            // Pack the first two chars into a u16 to match quickly.
            // Only apply this fast-path to ASCII; otherwise fall back to the previous tuple match semantics.
            let c1 = chars[0] as u32;
            let c2 = chars[1] as u32;
            if c1 <= 0x7F && c2 <= 0x7F {
                let pair = ((c1 as u16) << 8) | (c2 as u16);
                // Check against: cl, fl, bl, gl, sl, pl, br, pr, dr, st, sp, sk, and p* disallow list.
                match pair {
                    0x636C // cl
                    | 0x666C // fl
                    | 0x626C // bl
                    | 0x676C // gl
                    | 0x736C // sl
                    | 0x706C // pl
                    | 0x6272 // br
                    | 0x7072 // pr
                    | 0x6472 // dr
                    | 0x6672 // fr
                    | 0x6772 // gr
                    | 0x6B72 // kr
                    | 0x7374 // st
                    | 0x7370 // sp
                    | 0x736B // sk
                    | 0x7074 // pt
                    | 0x7063 // pc
                    | 0x7067 // pg
                    | 0x7071 // pq
                    | 0x7073 // ps
                    | 0x706B // pk
                    | 0x7064 // pd
                    | 0x7066 // pf
                    | 0x7062 // pb
                    => return true,
                    _ => {}
                }
            } else {
                // Fallback (non-ASCII)
                let c1 = chars[0];
                let c2 = chars[1];
                match (c1, c2) {
                    ('c', 'l')
                    | ('f', 'l')
                    | ('b', 'l')
                    | ('g', 'l')
                    | ('s', 'l')
                    | ('p', 'l')
                    | ('b', 'r')
                    | ('p', 'r')
                    | ('d', 'r')
                    | ('f', 'r')
                    | ('g', 'r')
                    | ('k', 'r')
                    | ('s', 't')
                    | ('s', 'p')
                    | ('s', 'k')
                    | ('p', 't')
                    | ('p', 'c')
                    | ('p', 'g')
                    | ('p', 'q')
                    | ('p', 's')
                    | ('p', 'k')
                    | ('p', 'd')
                    | ('p', 'f')
                    | ('p', 'b') => return true,
                    _ => {}
                }
            }
        }

        false
    }

    // New in-place tone application
    fn apply_tone_in_place(&self, chars: &mut [char], mask: u16, tone: u8) {
        let count = mask.count_ones();
        if count == 0 {
            return;
        }

        // If tone is 0 (z), we want to strip tones.
        // map_vowel_with_tone handles tone=0 by returning base char?
        // No, map_vowel_with_tone(c, 0) returns c.
        // If c is already 'á', we need to reset it to 'a'.
        // So we need a "remove_tone" function or map_vowel_with_tone needs to handle it.
        // Let's implement robust mapping.

        let target_pos = match count {
            1 => mask.trailing_zeros() as usize,
            2 => {
                let first = mask.trailing_zeros() as usize;
                let second = (mask & !(1 << first)).trailing_zeros() as usize;

                let f = chars.get(first).copied().unwrap_or('\0');
                let sc = chars.get(second).copied().unwrap_or('\0');

                // Standard open pairs that prefer tone on the first vowel
                // ia, ua, ưa, iu, eu (?), au, ao, ai, ay, eo, oi
                // EXCLUDED: oa, oe, uy, ui, uo (prefer second)
                let mut is_open_pair = (f == 'i' && (sc == 'a' || sc == 'u')) || // ia, iu
                    (f == 'u' && (sc == 'a' || sc == 'e')) || // ua (ue?)
                    (f == 'ư' && (sc == 'a' || sc == 'u')) || // ưa, ưu
                    (f == 'a' && (sc == 'o' || sc == 'e' || sc == 'i' || sc == 'u' || sc == 'y')) || // ao, ae, ai, au, ay
                    (f == 'e' && (sc == 'o' || sc == 'u')) || // eo, eu
                    (f == 'o' && sc == 'i') || // oi
                    (f == 'â' &&( sc == 'y' || sc == 'u')); // ây, âu

                // Exception: "qu" and "gi" logic
                // If starts with "qu" -> u is glide, tone on next vowel
                // If starts with "gi" -> i is consonant part, tone on next vowel
                if is_open_pair {
                    if chars.len() >= 2 {
                        let p0 = chars[0];
                        let p1 = chars[1];

                        // Check for 'qu' prefix where 'u' is the first vowel
                        if (p0 == 'q' || p0 == 'Q') && (p1 == 'u' || p1 == 'U') && first == 1 {
                            is_open_pair = false;
                        }
                        // Check for 'gi' prefix where 'i' is the first vowel
                        else if (p0 == 'g' || p0 == 'G') && (p1 == 'i' || p1 == 'I') && first == 1
                        {
                            is_open_pair = false;
                        }
                    }
                }

                if is_open_pair {
                    // Check coda: ký tự sau nguyên âm thứ 2
                    let has_coda = (second + 1) < chars.len();
                    if has_coda { second } else { first }
                } else {
                    second
                }
            }
            _ => (mask & !(1 << mask.trailing_zeros())).trailing_zeros() as usize,
        };

        if let Some(target) = chars.get_mut(target_pos) {
            *target = map_vowel_with_tone(*target, tone);
        }
    }
}

// Helpers
#[inline(always)]
fn classify(b: u8) -> u8 {
    CLASSIFY_TABLE[b as usize]
}

#[inline(always)]
fn is_w_target(b: u8) -> bool {
    W_TARGET_TABLE[b as usize]
}

#[inline(always)]
fn map_tone(b: u8) -> u8 {
    TONE_TABLE[b as usize]
}

const CLASSIFY_TABLE: [u8; 256] = {
    let mut t = [0u8; 256];
    t[b'a' as usize] = IS_VOWEL;
    t[b'e' as usize] = IS_VOWEL;
    t[b'o' as usize] = IS_VOWEL;
    t[b'u' as usize] = IS_VOWEL;
    t[b'i' as usize] = IS_VOWEL;
    t[b'y' as usize] = IS_VOWEL;

    t[b'w' as usize] = IS_MODIFIER;
    t[b'd' as usize] = IS_MODIFIER;

    t[b's' as usize] = IS_TONE_KEY;
    t[b'f' as usize] = IS_TONE_KEY;
    t[b'r' as usize] = IS_TONE_KEY;
    t[b'x' as usize] = IS_TONE_KEY;
    t[b'j' as usize] = IS_TONE_KEY;
    t[b'z' as usize] = IS_TONE_KEY;
    t
};

const W_TARGET_TABLE: [bool; 256] = {
    let mut t = [false; 256];
    t[b'a' as usize] = true;
    t[b'o' as usize] = true;
    t[b'u' as usize] = true;
    t[b'd' as usize] = true;
    t
};

const TONE_TABLE: [u8; 256] = {
    let mut t = [0u8; 256];
    t[b's' as usize] = 1;
    t[b'f' as usize] = 2;
    t[b'r' as usize] = 3;
    t[b'x' as usize] = 4;
    t[b'j' as usize] = 5;
    t[b'z' as usize] = 0;
    t
};

#[inline(always)]
fn resolve_telex(curr: u8, next: Option<u8>) -> (char, bool) {
    match (curr, next) {
        (b'a', Some(b'a')) => ('â', true),
        (b'a', Some(b'w')) => ('ă', true),
        (b'e', Some(b'e')) => ('ê', true),
        (b'o', Some(b'o')) => ('ô', true),
        (b'o', Some(b'w')) => ('ơ', true),
        (b'u', Some(b'w')) => ('ư', true),
        (b'd', Some(b'd')) => ('đ', true),
        (b'w', _) => ('ư', false),
        _ => (curr as char, false),
    }
}

#[inline(always)]
fn is_vowel_unicode(c: char) -> bool {
    matches!(
        c,
        'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'â' | 'ê' | 'ô' | 'ă' | 'ơ' | 'ư'
    )
}

fn map_vowel_with_tone(c: char, tone: u8) -> char {
    let base_id: Option<usize> = match c {
        'a' | 'á' | 'à' | 'ả' | 'ã' | 'ạ' => Some(0),
        'ă' | 'ắ' | 'ằ' | 'ẳ' | 'ẵ' | 'ặ' => Some(1),
        'â' | 'ấ' | 'ầ' | 'ẩ' | 'ẫ' | 'ậ' => Some(2),
        'e' | 'é' | 'è' | 'ẻ' | 'ẽ' | 'ẹ' => Some(3),
        'ê' | 'ế' | 'ề' | 'ể' | 'ễ' | 'ệ' => Some(4),
        'i' | 'í' | 'ì' | 'ỉ' | 'ĩ' | 'ị' => Some(5),
        'o' | 'ó' | 'ò' | 'ỏ' | 'õ' | 'ọ' => Some(6),
        'ô' | 'ố' | 'ồ' | 'ổ' | 'ỗ' | 'ộ' => Some(7),
        'ơ' | 'ớ' | 'ờ' | 'ở' | 'ỡ' | 'ợ' => Some(8),
        'u' | 'ú' | 'ù' | 'ủ' | 'ũ' | 'ụ' => Some(9),
        'ư' | 'ứ' | 'ừ' | 'ử' | 'ữ' | 'ự' => Some(10),
        'y' | 'ý' | 'ỳ' | 'ỷ' | 'ỹ' | 'ỵ' => Some(11),
        _ => None,
    };

    let Some(id) = base_id else {
        return c;
    };

    let t = if tone <= 5 { tone as usize } else { 0usize };
    TONE_VOWELS[id][t]
}

// Index: base vowel id (a,ă,â,e,ê,i,o,ô,ơ,u,ư,y) x tone (0..5)
// Tone: 0=base, 1=s, 2=f, 3=r, 4=x, 5=j
const TONE_VOWELS: [[char; 6]; 12] = [
    ['a', 'á', 'à', 'ả', 'ã', 'ạ'],
    ['ă', 'ắ', 'ằ', 'ẳ', 'ẵ', 'ặ'],
    ['â', 'ấ', 'ầ', 'ẩ', 'ẫ', 'ậ'],
    ['e', 'é', 'è', 'ẻ', 'ẽ', 'ẹ'],
    ['ê', 'ế', 'ề', 'ể', 'ễ', 'ệ'],
    ['i', 'í', 'ì', 'ỉ', 'ĩ', 'ị'],
    ['o', 'ó', 'ò', 'ỏ', 'õ', 'ọ'],
    ['ô', 'ố', 'ồ', 'ổ', 'ỗ', 'ộ'],
    ['ơ', 'ớ', 'ờ', 'ở', 'ỡ', 'ợ'],
    ['u', 'ú', 'ù', 'ủ', 'ũ', 'ụ'],
    ['ư', 'ứ', 'ừ', 'ử', 'ữ', 'ự'],
    ['y', 'ý', 'ỳ', 'ỷ', 'ỹ', 'ỵ'],
];

#[cfg(feature = "heapless")]
#[inline(always)]
fn new_raw_buffer() -> RawBuffer {
    RawBuffer::new()
}

#[cfg(feature = "heapless")]
#[inline(always)]
fn new_out_buffer() -> OutBuffer {
    OutBuffer::new()
}

#[cfg(not(feature = "heapless"))]
#[inline(always)]
fn new_raw_buffer() -> RawBuffer {
    String::with_capacity(32)
}

#[cfg(not(feature = "heapless"))]
#[inline(always)]
fn new_out_buffer() -> OutBuffer {
    String::with_capacity(128)
}
