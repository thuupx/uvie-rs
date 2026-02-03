// Constants Bitmask
const IS_VOWEL: u8 = 1 << 0;
const IS_MODIFIER: u8 = 1 << 1;
const IS_TONE_KEY: u8 = 1 << 2; // Keys: s, f, r, x, j, z

pub struct UltraFastViEngine {
    raw_buffer: String,
}

impl UltraFastViEngine {
    pub fn new() -> Self {
        Self { raw_buffer: String::with_capacity(32) }
    }

    pub fn feed(&mut self, key: char) -> String {
        if key.is_whitespace() {
            let res = self.render();
            self.raw_buffer.clear();
            return format!("{}{}", res, key);
        }
        self.raw_buffer.push(key.to_ascii_lowercase());
        self.render()
    }

    fn render(&self) -> String {
        if self.raw_buffer.is_empty() { return String::new(); }

        let bytes = self.raw_buffer.as_bytes();
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
                    if p_len < 32 { processed[p_len] = b; p_len += 1; }
                    continue;
                }

                // Rule 2: 'r' after 't' is 'tr'
                if b == b'r' && idx > 0 && bytes[idx-1] == b't' {
                    if p_len < 32 { processed[p_len] = b; p_len += 1; }
                    continue;
                }

                last_tone_char = b;
            } else {
                if p_len < 32 {
                    processed[p_len] = b;
                    p_len += 1;
                }
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
            if i + 2 < p_len && processed[i+1] == c && processed[i+2] == c {
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

        // 1.6. Retroactive 'w' bubbling
        // If we see 'w', bubble it leftwards until it hits a char it can modify (a, o, u, d)
        for k in 0..t_len {
            if toggled[k] == b'w' {
                // Check if there is a valid target to the left
                let mut has_target = false;
                for j in (0..k).rev() {
                     if is_w_target(toggled[j]) {
                         has_target = true;
                         break;
                     }
                }
                
                if has_target {
                    let mut cur = k;
                    while cur > 0 {
                        let prev = toggled[cur - 1];
                        if is_w_target(prev) {
                            break;
                        }
                        // Swap
                        toggled[cur] = prev;
                        toggled[cur - 1] = b'w';
                        cur -= 1;
                    }
                }
            }
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
            let next = if i + 1 < t_len { Some(toggled[i+1]) } else { None };

            let (mut c, consumed) = resolve_telex(curr, next);

            // Fix uow -> ươ
            // Logic: u + o + w -> ươ
            // EXCEPTION: if preceded by 'q', then it is qu + o + w -> qu + ơ
            if curr == b'u' && !consumed {
                if let Some(n) = next {
                    if n == b'o' {
                        if i + 2 < t_len && toggled[i+2] == b'w' {
                            // Check previous char for 'q'
                            let is_qu = if i > 0 {
                                let prev = toggled[i-1];
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
                if v_idx < 16 { vowel_mask |= 1 << v_idx; }
            }

            char_buf[c_len] = c;
            c_len += 1;
            v_idx += 1;

            i += if consumed { 2 } else { 1 };
        }

        // 3. Validation
        // Check invalid Vietnamese combinations (e.g. English words like "clear")
        if last_tone_char > 0 {
            let valid = !self.is_invalid_vietnamese_chars(&char_buf[..c_len], vowel_mask);
            if !valid {
                // If invalid, append tone char back and return
                char_buf[c_len] = last_tone_char as char;
                c_len += 1;
                return char_buf[..c_len].iter().collect();
            }
        } else {
             // If no tone char, but we want to ensure we don't return partial nonsense? 
             // actually, just return what we have.
        }

        // 4. Tone Placement
        if last_tone_char > 0 {
            let tone_id = map_tone(last_tone_char);
            // tone_id 0 means remove tone (z key)
            self.apply_tone_in_place(&mut char_buf[..c_len], vowel_mask, tone_id);
        }

        char_buf[..c_len].iter().collect()
    }

    fn is_invalid_vietnamese_chars(&self, chars: &[char], vowel_mask: u16) -> bool {
        if vowel_mask == 0 { return true; }
        
        // Simple string checks on chars
        // "ou" check:
        for i in 0..chars.len().saturating_sub(1) {
            if chars[i] == 'o' && chars[i+1] == 'u' { return true; }
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
            let c1 = chars[0];
            let c2 = chars[1];
            // Check against: cl, fl, bl, gl, sl, pl, br, pr, dr, st, sp, sk
            match (c1, c2) {
                ('c', 'l') | ('f', 'l') | ('b', 'l') | ('g', 'l') | ('s', 'l') | ('p', 'l') |
                ('b', 'r') | ('p', 'r') | ('d', 'r') |
                ('s', 't') | ('s', 'p') | ('s', 'k') => return true,
                _ => {}
            }
        }
        
        false
    }

    // New in-place tone application
    fn apply_tone_in_place(&self, chars: &mut [char], mask: u16, tone: u8) {
        let count = mask.count_ones();
        if count == 0 { return; }

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
                let mut is_open_pair =
                    (f == 'i' && (sc == 'a' || sc == 'u')) || // ia, iu
                    (f == 'u' && (sc == 'a' || sc == 'e')) || // ua (ue?)
                    (f == 'ư' && (sc == 'a' || sc == 'u')) || // ưa, ưu
                    (f == 'a' && (sc == 'o' || sc == 'e' || sc == 'i' || sc == 'u' || sc == 'y')) || // ao, ae, ai, au, ay
                    (f == 'e' && (sc == 'o' || sc == 'u')) || // eo, eu
                    (f == 'o' && sc == 'i') || // oi
                    (f == 'â' &&( sc == 'y' || sc == 'u'));   // ây, âu

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
                        else if (p0 == 'g' || p0 == 'G') && (p1 == 'i' || p1 == 'I') && first == 1 {
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
            },
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
    match b {
        b'a' | b'e' | b'o' | b'u' | b'i' | b'y' => IS_VOWEL,
        b'w' | b'd' => IS_MODIFIER,
        b's' | b'f' | b'r' | b'x' | b'j' | b'z' => IS_TONE_KEY,
        _ => 0,
    }
}

#[inline(always)]
fn is_w_target(b: u8) -> bool {
    matches!(b, b'a' | b'o' | b'u' | b'd')
}

#[inline(always)]
fn map_tone(b: u8) -> u8 {
    match b { b's'=>1, b'f'=>2, b'r'=>3, b'x'=>4, b'j'=>5, b'z'=>0, _=>0 }
}

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
    "aeiouyâêôăơư".contains(c)
}

fn map_vowel_with_tone(c: char, tone: u8) -> char {
    // If tone is 0, we must strip tone from c.
    let base = match c {
        'á'|'à'|'ả'|'ã'|'ạ' => 'a',
        'ắ'|'ằ'|'ẳ'|'ẵ'|'ặ' => 'ă',
        'ấ'|'ầ'|'ẩ'|'ẫ'|'ậ' => 'â',
        'é'|'è'|'ẻ'|'ẽ'|'ẹ' => 'e',
        'ế'|'ề'|'ể'|'ễ'|'ệ' => 'ê',
        'í'|'ì'|'ỉ'|'ĩ'|'ị' => 'i',
        'ó'|'ò'|'ỏ'|'õ'|'ọ' => 'o',
        'ố'|'ồ'|'ổ'|'ỗ'|'ộ' => 'ô',
        'ớ'|'ờ'|'ở'|'ỡ'|'ợ' => 'ơ',
        'ú'|'ù'|'ủ'|'ũ'|'ụ' => 'u',
        'ứ'|'ừ'|'ử'|'ữ'|'ự' => 'ư',
        'ý'|'ỳ'|'ỷ'|'ỹ'|'ỵ' => 'y',
        _ => c, // Already base or not a vowel
    };
    
    if tone == 0 { return base; }

    match (base, tone) {
        ('a', 1) => 'á', ('a', 2) => 'à', ('a', 3) => 'ả', ('a', 4) => 'ã', ('a', 5) => 'ạ',
        ('ă', 1) => 'ắ', ('ă', 2) => 'ằ', ('ă', 3) => 'ẳ', ('ă', 4) => 'ẵ', ('ă', 5) => 'ặ',
        ('â', 1) => 'ấ', ('â', 2) => 'ầ', ('â', 3) => 'ẩ', ('â', 4) => 'ẫ', ('â', 5) => 'ậ',
        ('e', 1) => 'é', ('e', 2) => 'è', ('e', 3) => 'ẻ', ('e', 4) => 'ẽ', ('e', 5) => 'ẹ',
        ('ê', 1) => 'ế', ('ê', 2) => 'ề', ('ê', 3) => 'ể', ('ê', 4) => 'ễ', ('ê', 5) => 'ệ',
        ('i', 1) => 'í', ('i', 2) => 'ì', ('i', 3) => 'ỉ', ('i', 4) => 'ĩ', ('i', 5) => 'ị',
        ('o', 1) => 'ó', ('o', 2) => 'ò', ('o', 3) => 'ỏ', ('o', 4) => 'õ', ('o', 5) => 'ọ',
        ('ô', 1) => 'ố', ('ô', 2) => 'ồ', ('ô', 3) => 'ổ', ('ô', 4) => 'ỗ', ('ô', 5) => 'ộ',
        ('ơ', 1) => 'ớ', ('ơ', 2) => 'ờ', ('ơ', 3) => 'ở', ('ơ', 4) => 'ỡ', ('ơ', 5) => 'ợ',
        ('u', 1) => 'ú', ('u', 2) => 'ù', ('u', 3) => 'ủ', ('u', 4) => 'ũ', ('u', 5) => 'ụ',
        ('ư', 1) => 'ứ', ('ư', 2) => 'ừ', ('ư', 3) => 'ử', ('ư', 4) => 'ữ', ('ư', 5) => 'ự',
        ('y', 1) => 'ý', ('y', 2) => 'ỳ', ('y', 3) => 'ỷ', ('y', 4) => 'ỹ', ('y', 5) => 'ỵ',
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::{UltraFastViEngine};

    fn type_seq(engine: &mut UltraFastViEngine, seq: &str) -> String {
        let mut out = String::new();
        for c in seq.chars() {
            out = engine.feed(c);
        }
        out
    }

    #[test]
    fn telex_modifier_basic() {
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "aa"), "â");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "aw"), "ă");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "ee"), "ê");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "oo"), "ô");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "ow"), "ơ");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "uw"), "ư");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "dd"), "đ");
    }

    #[test]
    fn tone_single_vowel_all_tones() {
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "as"), "á");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "af"), "à");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "ar"), "ả");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "ax"), "ã");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "aj"), "ạ");
    }

    #[test]
    fn z_key_removes_tone() {
        let mut e = UltraFastViEngine::new();
        // as -> á, z -> a
        assert_eq!(type_seq(&mut e, "asz"), "a");
        
        let mut e = UltraFastViEngine::new();
        // az -> a
        assert_eq!(type_seq(&mut e, "az"), "a");

        let mut e = UltraFastViEngine::new();
        // axz -> a
        assert_eq!(type_seq(&mut e, "axz"), "a");
    }

    #[test]
    fn toggling_triplet() {
        let mut e = UltraFastViEngine::new();
        // aaa -> a
        assert_eq!(type_seq(&mut e, "aaa"), "a");

        let mut e = UltraFastViEngine::new();
        // ddd -> d
        assert_eq!(type_seq(&mut e, "ddd"), "d");

        let mut e = UltraFastViEngine::new();
        // eee -> e
        assert_eq!(type_seq(&mut e, "eee"), "e");

         let mut e = UltraFastViEngine::new();
        // ooo -> o
        assert_eq!(type_seq(&mut e, "ooo"), "o");
    }

    #[test]
    fn tone_on_modified_vowels() {
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "aas"), "ấ");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "awj"), "ặ");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "ees"), "ế");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "oos"), "ố");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "ows"), "ớ");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "uws"), "ứ");
    }

    #[test]
    fn greedy_tone_last_wins() {
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "asf"), "à");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "afsj"), "ạ");
    }

    #[test]
    fn tone_placement_two_vowels_no_coda() {
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "hoas"), "hoá");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "hoaf"), "hoà");
    }

    #[test]
    fn tone_placement_two_vowels_with_coda() {
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "hoans"), "hoán");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "hoanj"), "hoạn");
    }

    #[test]
    fn tone_placement_three_vowels_targets_second_vowel() {
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "khuya"), "khuya");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "khuyas"), "khuýa");
    }

    #[test]
    fn whitespace_flushes_and_resets_buffer() {
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "aas"), "ấ");
        assert_eq!(e.feed(' '), "ấ ");
        assert_eq!(type_seq(&mut e, "as"), "á");
    }

    #[test]
    fn tone_only_input_produces_empty() {
        let mut e = UltraFastViEngine::new();
        // First char is treated as consonant
        assert_eq!(type_seq(&mut e, "s"), "s");
        
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "z"), "z");
    }

    #[test]
    fn do_not_apply_to_english() {
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "clear"), "clear");
        
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "flan"), "flan");
        
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "blob"), "blob");
    }

    #[test]
    fn special_uow_combo() {
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "huow"), "hươ");
        
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "huows"), "hướ");
    }
    
    #[test]
    fn valid_consonant_cluster() {
         let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "nghe"), "nghe");
        
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "nghes"), "nghé");
        
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "nghees"), "nghế");
    }

    #[test]
    fn regression_qu_gi_placement() {
        // qu + a -> quá (tone on a)
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "quas"), "quá");

        // qu + y -> quỳ (tone on y)
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "quyf"), "quỳ");

        // qu + i -> quỉ (tone on i)
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "quir"), "quỉ");

        // gi + a -> giá (tone on a)
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "gias"), "giá");
    }

    #[test]
    fn regression_vowel_pairs() {
        // oa -> hoà (tone on a, new style)
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "hoaf"), "hoà");

        // oe -> hoè (tone on e, new style)
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "hoef"), "hoè");

        // uy -> tuỳ (tone on y, new style)
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "tuyf"), "tuỳ");
        
        // ia -> mía (tone on i)
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "mias"), "mía");
        
        // ua -> múa (tone on u)
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "muas"), "múa");
        
        // ưa -> mứa (tone on ư)
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "muwas"), "mứa");
    }
}
