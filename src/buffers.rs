#[cfg(feature = "heapless")]
pub type RawBuffer = heapless::String<32>;

#[cfg(feature = "heapless")]
pub type OutBuffer = heapless::String<128>;

#[cfg(not(feature = "heapless"))]
pub type RawBuffer = String;

#[cfg(not(feature = "heapless"))]
pub type OutBuffer = String;

#[cfg(all(not(feature = "std"), not(feature = "heapless")))]
compile_error!(
    "no_std build requires `heapless` feature (use --no-default-features --features heapless)"
);

#[cfg(feature = "heapless")]
#[inline(always)]
pub fn new_raw_buffer() -> RawBuffer {
    RawBuffer::new()
}

#[cfg(feature = "heapless")]
#[inline(always)]
pub fn new_out_buffer() -> OutBuffer {
    OutBuffer::new()
}

#[cfg(not(feature = "heapless"))]
#[inline(always)]
pub fn new_raw_buffer() -> RawBuffer {
    String::with_capacity(32)
}

#[cfg(not(feature = "heapless"))]
#[inline(always)]
pub fn new_out_buffer() -> OutBuffer {
    String::with_capacity(128)
}
