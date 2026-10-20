//! Audio processing: decode, peak pyramid, frame stats (arch §3.1, §4.4).
//!
//! Everything here is pure/streaming math over PCM; the HTTP layer and the
//! job scheduler live elsewhere. Decode uses symphonia (pure Rust, no C deps).

pub mod decode;
pub mod frames;
pub mod peaks;
pub mod prepare;
