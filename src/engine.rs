use crate::buffers::{OutBuffer, RawBuffer, new_out_buffer, new_raw_buffer};
use crate::modes::{IS_TONE_KEY, InputMethod, Mode, mode_for};
use crate::tone::{is_vowel_unicode, map_vowel_with_tone};

pub struct UltraFastViEngine {
    raw_buffer: RawBuffer,
    out_buffer: OutBuffer,
    input_method: InputMethod,
    mode: &'static Mode,
}

impl UltraFastViEngine {
    pub fn new() -> Self {
        let input_method = InputMethod::Telex;
        Self {
            raw_buffer: new_raw_buffer(),
            out_buffer: new_out_buffer(),
            input_method,
            mode: mode_for(input_method),
        }
    }

    pub fn clear(&mut self) {
        self.raw_buffer.clear();
        self.out_buffer.clear();
    }

    pub fn set_input_method(&mut self, method: InputMethod) {
        self.input_method = method;
        self.mode = mode_for(method);
    }

    pub fn input_method(&self) -> InputMethod {
        self.input_method
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
        let mut p_len = 0usize;
        let mut last_tone_char = 0u8;

        // 1. Filter & Capture Tone
        for (idx, &b) in bytes.iter().enumerate() {
            let attr = self.mode.classify[b as usize];
            let is_tone = (attr & IS_TONE_KEY) != 0;

            if is_tone {
                // Rule 1: First character is always treated as consonant/content
                if idx == 0 {
                    processed[p_len] = b;
                    p_len += 1;
                    continue;
                }

                // Rule 2: 'r' after 't' is 'tr'
                // Extended: 'r' after 'p', 'f', 'c', 'b', 'd', 'g', 'k'
                if b == b'r' {
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
        let mut toggled = [0u8; 32];
        let mut t_len = 0usize;
        let mut i = 0usize;
        while i < p_len {
            let c = processed[i];
            if i + 2 < p_len && processed[i + 1] == c && processed[i + 2] == c {
                match c {
                    b'a' | b'e' | b'o' | b'd' => {
                        toggled[t_len] = c;
                        t_len += 1;
                        i += 3;
                        continue;
                    }
                    _ => {}
                }
            }
            toggled[t_len] = c;
            t_len += 1;
            i += 1;
        }

        // 1.6. Mode-dependent modifier preprocessing
        if self.mode.enable_w_bubbling {
            // Retroactive 'w' bubbling (single-pass insertion)
            let mut bubbled = [0u8; 32];
            let mut b_len = 0usize;
            let mut last_target_pos: Option<usize> = None;

            for k in 0..t_len {
                let c = toggled[k];
                if c == b'w' {
                    if let Some(tp) = last_target_pos {
                        if b_len < 32 {
                            let mut j = b_len;
                            while j > tp + 1 {
                                bubbled[j] = bubbled[j - 1];
                                j -= 1;
                            }
                            bubbled[tp + 1] = b'w';
                            b_len += 1;
                        }
                    } else if b_len < 32 {
                        bubbled[b_len] = b'w';
                        b_len += 1;
                    }
                } else {
                    bubbled[b_len] = c;
                    b_len += 1;
                    if self.mode.w_target[c as usize] {
                        last_target_pos = Some(b_len - 1);
                    }
                }
            }

            toggled = bubbled;
            t_len = b_len;
        }

        // 2. Resolve mode rules & Build Char Buffer
        let mut char_buf = ['\0'; 32];
        let mut c_len = 0usize;
        let mut vowel_mask = 0u16;
        let mut v_idx = 0usize;

        i = 0;
        while i < t_len {
            let curr = toggled[i];
            let next = if i + 1 < t_len {
                Some(toggled[i + 1])
            } else {
                None
            };

            let (mut c, consumed) = (self.mode.resolver)(curr, next);

            // Fix uow -> ươ (Telex-only behavior, but harmless if it never matches)
            if curr == b'u' && !consumed {
                if let Some(n) = next {
                    if n == b'o' {
                        if i + 2 < t_len && toggled[i + 2] == b'w' {
                            let is_qu = if i > 0 {
                                let prev = toggled[i - 1];
                                prev == b'q' || prev == b'Q'
                            } else {
                                false
                            };

                            if !is_qu {
                                c = 'ư';
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
        {
            let valid = !self.is_invalid_vietnamese_chars(&char_buf[..c_len], vowel_mask);
            if !valid {
                self.out_buffer.clear();
                let _ = self.out_buffer.push_str(&self.raw_buffer);
                return &self.out_buffer;
            }
        }

        // 4. Tone Placement
        if last_tone_char > 0 {
            let tone_id = self.mode.tone[last_tone_char as usize];
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
            return chars.len() > 1;
        }

        let mut mask_o: u32 = 0;
        let mut mask_u: u32 = 0;
        let mut idx: u32 = 0;
        for &c in chars.iter() {
            if idx >= 32 {
                break;
            }
            if c == 'o' {
                mask_o |= 1u32 << idx;
            } else if c == 'u' {
                mask_u |= 1u32 << idx;
            }
            idx += 1;
        }

        if (mask_o & (mask_u >> 1)) != 0 {
            return true;
        }

        let first_vowel_pos = vowel_mask.trailing_zeros() as usize;

        if first_vowel_pos >= 3 {
            if first_vowel_pos == 3 {
                if chars.len() >= 3 && chars[0] == 'n' && chars[1] == 'g' && chars[2] == 'h' {
                    return false;
                }
            }
            return true;
        }

        if first_vowel_pos == 2 {
            let c1 = chars[0] as u32;
            let c2 = chars[1] as u32;
            if c1 <= 0x7F && c2 <= 0x7F {
                let pair = ((c1 as u16) << 8) | (c2 as u16);
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

    fn apply_tone_in_place(&self, chars: &mut [char], mask: u16, tone: u8) {
        let count = mask.count_ones();
        if count == 0 {
            return;
        }

        let target_pos = match count {
            1 => mask.trailing_zeros() as usize,
            2 => {
                let first = mask.trailing_zeros() as usize;
                let second = (mask & !(1 << first)).trailing_zeros() as usize;

                let f = chars.get(first).copied().unwrap_or('\0');
                let sc = chars.get(second).copied().unwrap_or('\0');

                // Special case: ui/ưi (e.g. "túi", "gửi") place tone on the first vowel.
                // Without this, tone is incorrectly applied to 'i' (e.g. "gưỉ").
                // Exception: in "qu" prefix, 'u' is a glide, so tone belongs to the following vowel (e.g. "quỉ").
                let mut prefer_first = (f == 'u' || f == 'ư') && sc == 'i';

                // Standard open pairs that often prefer tone on the first vowel.
                // EXCLUDED pairs prefer tone on second vowel.
                let mut is_open_pair = (f == 'i' && (sc == 'a' || sc == 'u'))
                    || (f == 'u' && (sc == 'a' || sc == 'e'))
                    || (f == 'ư' && (sc == 'a' || sc == 'u'))
                    || (f == 'a'
                        && (sc == 'o' || sc == 'e' || sc == 'i' || sc == 'u' || sc == 'y'))
                    || (f == 'e' && (sc == 'o' || sc == 'u'))
                    || (f == 'o' && sc == 'i')
                    || (f == 'â' && (sc == 'y' || sc == 'u'));

                // Exception: "qu" and "gi" logic
                // If starts with "qu" -> u is glide, tone on next vowel
                // If starts with "gi" -> i is consonant part, tone on next vowel
                if chars.len() >= 2 {
                    let p0 = chars[0];
                    let p1 = chars[1];

                    if (p0 == 'q' || p0 == 'Q') && (p1 == 'u' || p1 == 'U') && first == 1 {
                        is_open_pair = false;
                        prefer_first = false;
                    } else if (p0 == 'g' || p0 == 'G')
                        && (p1 == 'i' || p1 == 'I')
                        && first == 1
                    {
                        is_open_pair = false;
                        prefer_first = false;
                    }
                }

                if prefer_first {
                    first
                } else if is_open_pair {
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
