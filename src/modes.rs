pub const IS_VOWEL: u8 = 1 << 0;
pub const IS_MODIFIER: u8 = 1 << 1;
pub const IS_TONE_KEY: u8 = 1 << 2;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputMethod {
    Telex,
    Vni,
}

type ResolverFn = fn(u8, Option<u8>) -> (char, bool);

pub struct Mode {
    pub classify: &'static [u8; 256],
    pub tone: &'static [u8; 256],
    pub w_target: &'static [bool; 256],
    pub resolver: ResolverFn,
    pub enable_w_bubbling: bool,
}

pub fn mode_for(method: InputMethod) -> &'static Mode {
    match method {
        InputMethod::Telex => &TELEX_MODE,
        InputMethod::Vni => &VNI_MODE,
    }
}

const TELEX_MODE: Mode = Mode {
    classify: &CLASSIFY_TELEX,
    tone: &TONE_TELEX,
    w_target: &W_TARGET_TELEX,
    resolver: resolve_telex,
    enable_w_bubbling: true,
};

const VNI_MODE: Mode = Mode {
    classify: &CLASSIFY_VNI,
    tone: &TONE_VNI,
    w_target: &W_TARGET_VNI,
    resolver: resolve_vni,
    enable_w_bubbling: false,
};

pub const CLASSIFY_TELEX: [u8; 256] = {
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

pub const CLASSIFY_VNI: [u8; 256] = {
    let mut t = [0u8; 256];
    t[b'a' as usize] = IS_VOWEL;
    t[b'e' as usize] = IS_VOWEL;
    t[b'o' as usize] = IS_VOWEL;
    t[b'u' as usize] = IS_VOWEL;
    t[b'i' as usize] = IS_VOWEL;
    t[b'y' as usize] = IS_VOWEL;

    t[b'0' as usize] = IS_TONE_KEY;
    t[b'1' as usize] = IS_TONE_KEY;
    t[b'2' as usize] = IS_TONE_KEY;
    t[b'3' as usize] = IS_TONE_KEY;
    t[b'4' as usize] = IS_TONE_KEY;
    t[b'5' as usize] = IS_TONE_KEY;
    t
};

pub const W_TARGET_TELEX: [bool; 256] = {
    let mut t = [false; 256];
    t[b'a' as usize] = true;
    t[b'o' as usize] = true;
    t[b'u' as usize] = true;
    t[b'd' as usize] = true;
    t
};

pub const W_TARGET_VNI: [bool; 256] = [false; 256];

pub const TONE_TELEX: [u8; 256] = {
    let mut t = [0u8; 256];
    t[b's' as usize] = 1;
    t[b'f' as usize] = 2;
    t[b'r' as usize] = 3;
    t[b'x' as usize] = 4;
    t[b'j' as usize] = 5;
    t[b'z' as usize] = 0;
    t
};

pub const TONE_VNI: [u8; 256] = {
    let mut t = [0u8; 256];
    t[b'0' as usize] = 0;
    t[b'1' as usize] = 1;
    t[b'2' as usize] = 2;
    t[b'3' as usize] = 3;
    t[b'4' as usize] = 4;
    t[b'5' as usize] = 5;
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
fn resolve_vni(curr: u8, next: Option<u8>) -> (char, bool) {
    match (curr, next) {
        (b'a', Some(b'6')) => ('â', true),
        (b'a', Some(b'8')) => ('ă', true),
        (b'e', Some(b'6')) => ('ê', true),
        (b'o', Some(b'6')) => ('ô', true),
        (b'o', Some(b'7')) => ('ơ', true),
        (b'u', Some(b'7')) => ('ư', true),
        (b'd', Some(b'9')) => ('đ', true),
        _ => (curr as char, false),
    }
}
