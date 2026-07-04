//! `qu` / `gi` glide detection and character-class helpers.

/// Returns `true` if the onset is `qu` (so the following `u` is a glide, not
/// a nucleus vowel).
#[inline]
pub fn onset_is_qu(onset_raw: &[u8]) -> bool {
    onset_raw == b"qu"
}

/// Returns `true` if the onset is `gi` (so the following `i` is a glide, not
/// a nucleus vowel). `d` is the raw key for `đ` as onset - `gi` raw is `gi`.
#[inline]
pub fn onset_is_gi(onset_raw: &[u8]) -> bool {
    onset_raw == b"gi"
}

/// Returns `true` if `c` is a Vietnamese vowel base character (resolved).
#[inline]
pub fn is_vowel_base(c: char) -> bool {
    matches!(
        c,
        'a' | 'ă' | 'â' | 'e' | 'ê' | 'i' | 'o' | 'ô' | 'ơ' | 'u' | 'ư' | 'y'
    )
}
