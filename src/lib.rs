// Constants Bitmask
const IS_VOWEL: u8 = 1 << 0;
const IS_MODIFIER: u8 = 1 << 1;
const IS_TONE_KEY: u8 = 1 << 2; // Các phím s, f, r, x, j

pub struct UltraFastViEngine {
    raw_buffer: String,
}

impl UltraFastViEngine {
    pub fn new() -> Self {
        Self { raw_buffer: String::with_capacity(16) }
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
        let mut processed = [0u8; 16];
        let mut p_len = 0;
        let mut last_tone_char = 0u8;

        // 1. Single Pass: Tách tone char thông minh hơn
        for (idx, &b) in bytes.iter().enumerate() {
            let attr = classify(b);
            let is_tone = (attr & IS_TONE_KEY) != 0;

            if is_tone {
                // Rule 1: Ký tự đầu tiên luôn là phụ âm
                if idx == 0 {
                    if p_len < 16 { processed[p_len] = b; p_len += 1; }
                    continue;
                }

                // Rule 2: 'r' đi sau 't' là phụ âm 'tr'
                if b == b'r' && bytes[idx-1] == b't' {
                    if p_len < 16 { processed[p_len] = b; p_len += 1; }
                    continue;
                }

                // Còn lại là dấu thanh
                last_tone_char = b;
            } else {
                if p_len < 16 {
                    processed[p_len] = b;
                    p_len += 1;
                }
            }
        }

        // 2. Resolve Telex (biến đổi nguyên âm/đ)
        let mut core_syllable = String::with_capacity(16);
        let mut i = 0;
        let mut vowel_mask = 0u16;
        let mut v_idx = 0;

        while i < p_len {
            let curr = processed[i];
            let next = processed.get(i + 1).copied();

            let (mut c, consumed) = resolve_telex(curr, next);

            // Fix uow -> ươ
            if curr == b'u' && !consumed {
                if let Some(n) = next {
                    if n == b'o' {
                        if let Some(nn) = processed.get(i + 2) {
                            if *nn == b'w' { c = 'ư'; }
                        }
                    }
                }
            }

            if is_vowel_unicode(c) {
                vowel_mask |= 1 << v_idx;
            }

            core_syllable.push(c);
            i += if consumed { 2 } else { 1 };
            v_idx += 1;
        }

        // 3. Validation Logic (Core Only)
        if last_tone_char > 0 && self.is_invalid_vietnamese(&core_syllable, vowel_mask) {
            core_syllable.push(last_tone_char as char);
            return core_syllable;
        }

        // 4. Tone Placement
        if last_tone_char > 0 {
            let tone_id = map_tone(last_tone_char);
            self.apply_tone_bitwise(&mut core_syllable, vowel_mask, tone_id)
        } else {
            core_syllable
        }
    }

    fn is_invalid_vietnamese(&self, s: &str, vowel_mask: u16) -> bool {
        // ... (Giữ nguyên logic validation đã viết ở câu trước)
        if vowel_mask == 0 { return true; }
        // Fix sound
        if s.contains("ou") { return true; }

        let first_vowel_pos = vowel_mask.trailing_zeros() as usize;
        if first_vowel_pos >= 3 {
            if first_vowel_pos == 3 && s.starts_with("ngh") { return false; }
            return true;
        }
        if first_vowel_pos == 2 {
            let bytes = s.as_bytes();
            let pair = [bytes[0], bytes[1]];
            if matches!(&pair,
                b"cl" | b"fl" | b"bl" | b"gl" | b"sl" | b"pl" | b"br" | b"pr" | b"dr" | b"st" | b"sp" | b"sk"
            ) { return true; }
        }
        false
    }

    fn apply_tone_bitwise(&self, s: &mut String, mask: u16, tone: u8) -> String {
        let count = mask.count_ones();
        if count == 0 { return s.clone(); }

        // Chuyển sang Vec<char> để truy cập đúng ký tự Unicode (mâ, mây...)
        let mut chars: Vec<char> = s.chars().collect();

        let target_pos = match count {
            1 => mask.trailing_zeros() as usize,
            2 => {
                let first = mask.trailing_zeros() as usize;
                // Xóa bit đầu tiên để tìm bit thứ 2
                let second = (mask & !(1 << first)).trailing_zeros() as usize;

                let f = chars.get(first).copied().unwrap_or('\0');
                let sc = chars.get(second).copied().unwrap_or('\0');

                // Danh sách các cặp nguyên âm ưu tiên dấu ở vị trí 1 khi không có đuôi
                // (Bao gồm: oa, oe, uy, uo, ao, ae, ai, au, oi, eo, ay, ây, ui)
                let is_open_pair =
                    (f == 'o' && (sc == 'a' || sc == 'e' || sc == 'i')) || // oa, oe, oi
                        (f == 'u' && (sc == 'y' || sc == 'o' || sc == 'i')) || // uy, uo, ui
                        (f == 'a' && (sc == 'o' || sc == 'e' || sc == 'i' || sc == 'u' || sc == 'y')) || // ao, ae, ai, au, ay
                        (f == 'e' && sc == 'o') || // eo
                        (f == 'â' &&( sc == 'y' || sc == 'u')) ||   // ây
                        (f == 'i' && sc == 'a');


                if is_open_pair {
                    // Check coda: ký tự sau nguyên âm thứ 2
                    let has_coda = (second + 1) < chars.len();
                    if has_coda { second } else { first }
                } else {
                    second // Default (ua, ia, ưa...)
                }
            },
            // 3 nguyên âm (khuỷu) -> vào giữa
            _ => (mask & !(1 << mask.trailing_zeros())).trailing_zeros() as usize,
        };

        // Inject tone trực tiếp vào Vec<char> rồi collect lại
        if let Some(target) = chars.get_mut(target_pos) {
            *target = map_vowel_with_tone(*target, tone);
        }
        chars.into_iter().collect()
    }

}
// Helpers
#[inline(always)]
fn classify(b: u8) -> u8 {
    match b {
        b'a' | b'e' | b'o' | b'u' | b'i' | b'y' => IS_VOWEL,
        b'w' | b'd' => IS_MODIFIER,
        b's' | b'f' | b'r' | b'x' | b'j' => IS_TONE_KEY,
        _ => 0,
    }
}

#[inline(always)]
fn map_tone(b: u8) -> u8 {
    match b { b's'=>1, b'f'=>2, b'r'=>3, b'x'=>4, b'j'=>5, _=>0 }
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

fn inject_unicode_tone(s: &str, idx: usize, tone: u8) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    if let Some(target) = chars.get_mut(idx) {
        *target = map_vowel_with_tone(*target, tone);
    }
    chars.into_iter().collect()
}

fn map_vowel_with_tone(c: char, tone: u8) -> char {
    match (c, tone) {
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
        assert_eq!(type_seq(&mut e, "hoas"), "hóa");

        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "hoaf"), "hòa");
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
        assert_eq!(type_seq(&mut e, "s"), "");
    }

    #[test]
    fn do_not_apply_to_english() {
        let mut e = UltraFastViEngine::new();
        assert_eq!(type_seq(&mut e, "clear"), "clear");
    }
}
