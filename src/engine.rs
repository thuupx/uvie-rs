use crate::buffers::{OutBuffer, RawBuffer, new_out_buffer, new_raw_buffer};
use crate::modes::{IS_TONE_KEY, InputMethod, Mode, mode_for};
use crate::tone::{is_vowel_unicode, map_vowel_with_tone};

/// Bitmask lookup table for invalid Vietnamese consonant pairs.
/// Index = (c1 - b'a') * 26 + (c2 - b'a'), value = true if pair is invalid.
static INVALID_PAIR_TABLE: [bool; 676] = {
    let mut t = [false; 676];
    // Helper: encode pair as index
    macro_rules! mark {
        ($a:expr, $b:expr) => {
            t[($a - b'a') as usize * 26 + ($b - b'a') as usize] = true;
        };
    }
    mark!(b'c', b'l'); mark!(b'f', b'l'); mark!(b'b', b'l'); mark!(b'g', b'l');
    mark!(b's', b'l'); mark!(b'p', b'l');
    mark!(b'b', b'r'); mark!(b'p', b'r'); mark!(b'd', b'r'); mark!(b'f', b'r');
    mark!(b'g', b'r'); mark!(b'k', b'r');
    mark!(b's', b't'); mark!(b's', b'p'); mark!(b's', b'k');
    mark!(b'p', b't'); mark!(b'p', b'c'); mark!(b'p', b'g'); mark!(b'p', b'q');
    mark!(b'p', b's'); mark!(b'p', b'k'); mark!(b'p', b'd'); mark!(b'p', b'f');
    mark!(b'p', b'b');
    t
};

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

        // Filter tone + Toggling (ddd -> d) in one pass
        let mut toggled = [0u8; 32];
        let mut t_len = 0usize;
        let mut last_tone_char = 0u8;
        let mut tone_cancelled = false;
        // State for toggling: track consecutive count of the current character
        let mut run_char: u8 = 0;
        let mut run_count: u8 = 0;
        // Flags for deferred bubbling (computed during this pass, zero extra cost)
        let mut seen_mod: u8 = 0;   // bitmask: bit0=a, bit1=e, bit2=o, bit3=d
        let mut need_mod_bubble = false;
        let mut has_w = false;

        for (idx, &b) in bytes.iter().enumerate() {
            let attr = self.mode.classify[b as usize];
            let is_tone = (attr & IS_TONE_KEY) != 0;

            if is_tone {
                // Rule 1: First character is always treated as consonant/content
                if idx == 0 {
                    run_char = b;
                    run_count = 1;
                    toggled[t_len] = b;
                    t_len += 1;
                    continue;
                }

                // Rule 2: 'r' after 't' is 'tr'
                // Extended: 'r' after 'p', 'f', 'c', 'b', 'd', 'g', 'k'
                if b == b'r' {
                    let prev = bytes[idx - 1];
                    if matches!(prev, b't' | b'p' | b'f' | b'c' | b'b' | b'd' | b'g' | b'k') {
                        run_char = b;
                        run_count = 1;
                        toggled[t_len] = b;
                        t_len += 1;
                        continue;
                    }
                }

                // Double tone key cancellation: ss, ff, rr, xx, jj -> undo tone, put key back as literal
                if b == last_tone_char {
                    // Cancel the tone and re-insert the key as a literal
                    if t_len < 32 {
                        toggled[t_len] = b;
                        t_len += 1;
                    }
                    last_tone_char = 0;
                    tone_cancelled = true;
                } else {
                    // If tone was previously cancelled and we see a new tone key,
                    // don't re-apply tone (the user already cancelled)
                    if tone_cancelled {
                        if t_len < 32 {
                            toggled[t_len] = b;
                            t_len += 1;
                        }
                    } else {
                        last_tone_char = b;
                    }
                }
            } else {
                // Fused toggling: detect triple-repeat (aaa->a, ddd->d, etc.)
                if b == run_char {
                    run_count += 1;
                    if run_count == 3 && matches!(b, b'a' | b'e' | b'o' | b'd') {
                        t_len -= 1;
                        run_count = 1;
                        continue;
                    }
                } else {
                    run_char = b;
                    run_count = 1;
                }
                // Track modifier/w flags for deferred bubbling (zero-cost: piggyback on this loop)
                match b {
                    b'a' => { let bit = 1u8 << 0; if seen_mod & bit != 0 { need_mod_bubble = true; } seen_mod |= bit; }
                    b'e' => { let bit = 1u8 << 1; if seen_mod & bit != 0 { need_mod_bubble = true; } seen_mod |= bit; }
                    b'o' => { let bit = 1u8 << 2; if seen_mod & bit != 0 { need_mod_bubble = true; } seen_mod |= bit; }
                    b'd' => { let bit = 1u8 << 3; if seen_mod & bit != 0 { need_mod_bubble = true; } seen_mod |= bit; }
                    b'w' => { has_w = true; }
                    _ => {}
                }
                toggled[t_len] = b;
                t_len += 1;
            }
        }

        // Fused modifier + w bubbling pass (single buffer copy)
        // Handles: free-style modifier bubbling (aa/ee/oo/dd), double-w cancellation, w-bubbling
        // Flags need_mod_bubble / has_w were computed in the first pass above (zero extra scan)
        const W_LITERAL: u8 = 0x01;
        let need_w_pass = has_w && self.mode.enable_w_bubbling;
        {
            if need_mod_bubble || need_w_pass {
                let mut buf = [0u8; 32];
                let mut b_len = 0usize;

                // Phase 1: modifier bubbling + double-w collapse in one scan
                let mut last_pos: [u8; 4] = [0xFF; 4]; // a,e,o,d positions (0xFF = none)
                let mut wi = 0usize;
                while wi < t_len {
                    let c = toggled[wi];

                    // Double-w cancellation
                    if c == b'w' && self.mode.enable_w_bubbling {
                        if wi + 1 < t_len && toggled[wi + 1] == b'w' {
                            buf[b_len] = W_LITERAL;
                            b_len += 1;
                            wi += 2;
                            continue;
                        }
                        // Single w: just append, will be bubbled in phase 2
                        buf[b_len] = c;
                        b_len += 1;
                        wi += 1;
                        continue;
                    }

                    // Modifier bubbling for a,e,o,d
                    let slot = match c {
                        b'a' => Some(0),
                        b'e' => Some(1),
                        b'o' => Some(2),
                        b'd' => Some(3),
                        _ => None,
                    };

                    if let Some(s) = slot {
                        if last_pos[s] != 0xFF {
                            // Bubble: insert next to first occurrence
                            let insert_at = last_pos[s] as usize + 1;
                            buf.copy_within(insert_at..b_len, insert_at + 1);
                            buf[insert_at] = c;
                            b_len += 1;
                            last_pos[s] = 0xFF; // consumed
                            // Shift tracked positions
                            for p in last_pos.iter_mut() {
                                if *p != 0xFF && *p as usize >= insert_at {
                                    *p += 1;
                                }
                            }
                        } else {
                            last_pos[s] = b_len as u8;
                            buf[b_len] = c;
                            b_len += 1;
                        }
                    } else {
                        buf[b_len] = c;
                        b_len += 1;
                    }
                    wi += 1;
                }

                // Phase 2: w-bubbling in-place on buf (only if needed)
                if need_w_pass {
                    let mut out = [0u8; 32];
                    let mut o_len = 0usize;
                    let mut last_target_pos: Option<usize> = None;

                    for k in 0..b_len {
                        let c = buf[k];
                        if c == b'w' {
                            if let Some(tp) = last_target_pos {
                                let insert_at = tp + 1;
                                out.copy_within(insert_at..o_len, insert_at + 1);
                                out[insert_at] = b'w';
                                o_len += 1;
                            } else {
                                out[o_len] = b'w';
                                o_len += 1;
                            }
                        } else {
                            out[o_len] = c;
                            o_len += 1;
                            if self.mode.w_target[c as usize] {
                                last_target_pos = Some(o_len - 1);
                            }
                        }
                    }
                    toggled = out;
                    t_len = o_len;
                } else {
                    toggled = buf;
                    t_len = b_len;
                }
            }
        }

        // Resolve mode rules & Build Char Buffer
        let mut char_buf = ['\0'; 32];
        let mut c_len = 0usize;
        let mut vowel_mask = 0u16;

        let mut i = 0usize;
        while i < t_len {
            let curr = toggled[i];

            // W_LITERAL sentinel: output literal 'w', skip resolver
            if curr == W_LITERAL {
                char_buf[c_len] = 'w';
                c_len += 1;
                i += 1;
                continue;
            }

            let next = if i + 1 < t_len {
                Some(toggled[i + 1])
            } else {
                None
            };

            let (mut c, consumed) = (self.mode.resolver)(curr, next);

            // uow -> ươ
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
                if c_len < 16 {
                    vowel_mask |= 1 << c_len;
                }
            }

            char_buf[c_len] = c;
            c_len += 1;

            i += if consumed { 2 } else { 1 };
        }

        // If no vowels in the resolved output and tone keys were stripped, fall back to raw
        // This handles cases like "txt", "sx" where tone keys have no vowel to act on
        // Exception: if a modifier was applied (e.g. dd -> đ), keep the resolved output
        if vowel_mask == 0 && last_tone_char != 0 && !tone_cancelled {
            let has_modified = char_buf[..c_len].iter().any(|&c| !c.is_ascii());
            if !has_modified {
                self.out_buffer.clear();
                let _ = self.out_buffer.push_str(&self.raw_buffer);
                return &self.out_buffer;
            }
        }

        // Validation
        if self.is_invalid_vietnamese_chars(&char_buf[..c_len], vowel_mask) {
            self.out_buffer.clear();
            let _ = self.out_buffer.push_str(&self.raw_buffer);
            return &self.out_buffer;
        }

        // Tone Placement
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
            // Both chars must be lowercase ASCII a-z for table lookup
            if c1 >= b'a' as u32 && c1 <= b'z' as u32 && c2 >= b'a' as u32 && c2 <= b'z' as u32 {
                let table_idx = (c1 - b'a' as u32) as usize * 26 + (c2 - b'a' as u32) as usize;
                if INVALID_PAIR_TABLE[table_idx] {
                    return true;
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
                // Exception: in "qu" prefix, 'u' is a glide, so tone belongs to the following vowel.
                let mut prefer_first = (f == 'u' || f == 'ư') && sc == 'i';

                // Modified/circumflex vowels paired with a plain vowel: tone on the modified vowel.
                // e.g. ơi(mới), ôi(tối), êu(nếu), âu(đầu), ây(đấy), âo(cháo/nấo)
                // Exception: ươ pair — tone goes on ơ (second), not ư.
                let f_is_modified = matches!(f, 'ơ' | 'ô' | 'ê' | 'â' | 'ă');
                let sc_is_plain = matches!(sc, 'a' | 'e' | 'i' | 'o' | 'u' | 'y');
                if f_is_modified && sc_is_plain {
                    prefer_first = true;
                }

                // Standard open pairs that often prefer tone on the first vowel.
                let mut is_open_pair = (f == 'i' && (sc == 'a' || sc == 'u'))
                    || (f == 'u' && (sc == 'a' || sc == 'e'))
                    || (f == 'ư' && (sc == 'a' || sc == 'u'))
                    || (f == 'a'
                        && (sc == 'o' || sc == 'e' || sc == 'i' || sc == 'u' || sc == 'y'))
                    || (f == 'e' && (sc == 'o' || sc == 'u'))
                    || (f == 'o' && sc == 'i')
                    || (f == 'â' && (sc == 'y' || sc == 'u'));

                // Exception: "qu" and "gi" logic
                if chars.len() >= 2 {
                    let p0 = chars[0];
                    let p1 = chars[1];

                    if (p0 == 'q' || p0 == 'Q') && (p1 == 'u' || p1 == 'U') && first == 1 {
                        is_open_pair = false;
                        prefer_first = false;
                    } else if (p0 == 'g' || p0 == 'G') && (p1 == 'i' || p1 == 'I') && first == 1 {
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
