//! Per-10 ms loudness frames (arch §3.1, §4.3).
//!
//! During prepare we compute, for every 10 ms window, `peak_db` (max |x|
//! across the window and all channels) and `rms_db` (RMS over the window).
//! Detection later is a pure in-memory scan over these cached frames (D28),
//! so re-runs are milliseconds with no re-decode.

use crate::error::ApiError;

/// Floor for dB values so nothing is `-inf` (arch §4.3: −100 dB guard).
pub const DB_FLOOR: f32 = -100.0;

/// Frame stats for one 10 ms window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStats {
    pub peak_db: f32,
    pub rms_db: f32,
}

/// Streaming accumulator: feed interleaved PCM chunks, get per-10 ms frames.
pub struct FrameAccumulator {
    /// Interleaved samples per 10 ms window (`round(sr/100) × channels`).
    window_samples: usize,
    /// Current window's running max |sample| and sum of squares.
    peak_abs: f32,
    sum_sq: f64,
    filled: usize,
    channels: usize,
    frames: Vec<FrameStats>,
}

impl FrameAccumulator {
    /// `sample_rate` and `channels` define the 10 ms window: each frame spans
    /// `round(sample_rate / 100)` sample frames across all channels, i.e.
    /// `samples_per_frame × channels` interleaved samples.
    pub fn new(sample_rate: u32, channels: usize) -> Self {
        let channels = channels.max(1);
        let window_samples = samples_per_frame(sample_rate) * channels;
        Self {
            window_samples,
            peak_abs: 0.0,
            sum_sq: 0.0,
            filled: 0,
            channels,
            frames: Vec::new(),
        }
    }

    /// Feed one interleaved PCM chunk (any length; frames are completed and
    /// flushed as they fill). `samples.len()` must be a multiple of `channels`.
    pub fn push_interleaved_f32(&mut self, samples: &[f32]) {
        debug_assert_eq!(samples.len() % self.channels, 0);
        for &sample in samples {
            let abs = sample.abs();
            if abs > self.peak_abs {
                self.peak_abs = abs;
            }
            self.sum_sq += (sample as f64) * (sample as f64);
            self.filled += 1;
            if self.filled == self.window_samples {
                self.flush_frame();
            }
        }
    }

    fn flush_frame(&mut self) {
        let n = self.filled as f64;
        let rms = (self.sum_sq / n).sqrt() as f32;
        self.frames.push(FrameStats {
            peak_db: amplitude_to_db(self.peak_abs),
            rms_db: amplitude_to_db(rms),
        });
        self.peak_abs = 0.0;
        self.sum_sq = 0.0;
        self.filled = 0;
    }

    /// Finish, flushing a trailing partial frame if any samples were seen.
    pub fn finish(mut self) -> Vec<FrameStats> {
        if self.filled > 0 {
            self.flush_frame();
        }
        self.frames
    }

    /// Frames computed so far (excluding a trailing partial frame).
    pub fn frames(&self) -> &[FrameStats] {
        &self.frames
    }
}

/// Samples per 10 ms frame for one channel (arch §4.3: `round(sr / 100)`).
#[inline]
pub fn samples_per_frame(sample_rate: u32) -> usize {
    ((sample_rate as f64) / 100.0).round() as usize
}

/// Linear amplitude → dBFS with a −100 dB floor (arch §4.3).
#[inline]
pub fn amplitude_to_db(amplitude: f32) -> f32 {
    if amplitude <= 0.0 {
        return DB_FLOOR;
    }
    let db = 20.0 * amplitude.log10();
    if db < DB_FLOOR {
        DB_FLOOR
    } else {
        db
    }
}

/// 10th-percentile of `values` (arch §4.3: noise floor). Returns `None` when
/// `values` is empty.
pub fn percentile10(values: &[f32]) -> Option<f32> {
    percentile(values, 0.10)
}

/// Linear-interpolated percentile (0.0..=1.0) of `values`. Returns `None` when
/// `values` is empty.
pub fn percentile(values: &[f32], q: f64) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in frame data"));
    let rank = q * (sorted.len() as f64 - 1.0);
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        Some(sorted[lo])
    } else {
        let weight = (rank - lo as f64) as f32;
        Some(sorted[lo] * (1.0 - weight) + sorted[hi] * weight)
    }
}

/// Suggested detection threshold: noise floor + 12 dB, clamped to
/// [−80, −10] (arch §4.3, FR-11).
pub fn suggested_threshold(noise_floor_db: f32) -> f32 {
    (noise_floor_db + 12.0).clamp(-80.0, -10.0)
}

/// Validate prepare inputs before the expensive decode pass.
pub fn validate_sample_rate(sample_rate: u32) -> Result<(), ApiError> {
    if sample_rate == 0 {
        return Err(ApiError::CorruptFile);
    }
    if samples_per_frame(sample_rate) == 0 {
        return Err(ApiError::CorruptFile);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms_of_constant(value: f32) -> f32 {
        amplitude_to_db(value) // sanity: for a constant, peak == rms in dB
    }

    #[test]
    fn samples_per_frame_rounds_to_tenth_of_second() {
        assert_eq!(samples_per_frame(44_100), 441);
        assert_eq!(samples_per_frame(48_000), 480);
        assert_eq!(samples_per_frame(8_000), 80);
    }

    #[test]
    fn amplitude_to_db_has_floor_and_unity_reference() {
        assert_eq!(amplitude_to_db(0.0), DB_FLOOR);
        assert_eq!(amplitude_to_db(-1.0), DB_FLOOR, "abs taken");
        assert!((amplitude_to_db(1.0) - 0.0).abs() < 1e-6);
        assert!((amplitude_to_db(0.5) - (-6.0206)).abs() < 1e-3);
        assert_eq!(amplitude_to_db(1e-9), DB_FLOOR, "below floor clamps");
    }

    #[test]
    fn accumulator_frames_full_windows_then_partial() {
        // 441 samples/frame at 44.1 kHz → feed 1.5 frames of constant 1.0.
        let mut acc = FrameAccumulator::new(44_100, 1);
        let samples = vec![1.0; 661];
        acc.push_interleaved_f32(&samples);
        assert_eq!(acc.frames().len(), 1, "one full frame flushed mid-stream");
        let frames = acc.finish();
        assert_eq!(frames.len(), 2, "trailing partial frame flushed on finish");
        // Constant 1.0 → peak and rms both 0 dBFS.
        assert!((frames[0].peak_db - 0.0).abs() < 1e-4);
        assert!((frames[0].rms_db - 0.0).abs() < 1e-4);
        let _ = rms_of_constant(1.0);
    }

    #[test]
    fn silence_yields_floor_db() {
        let mut acc = FrameAccumulator::new(44_100, 1);
        acc.push_interleaved_f32(&vec![0.0; 441]);
        let frames = acc.finish();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].peak_db, DB_FLOOR);
        assert_eq!(frames[0].rms_db, DB_FLOOR);
    }

    #[test]
    fn peak_covers_all_channels_in_interleaved_window() {
        // Two channels: alternating loud/quiet; peak must see the loud one.
        let mut acc = FrameAccumulator::new(44_100, 2);
        let mut samples = vec![0.0f32; 882]; // 441 frames × 2 channels
        for (idx, sample) in samples.iter_mut().enumerate() {
            if idx % 2 == 0 {
                *sample = 1.0; // channel A loud
            } // channel B silent
        }
        acc.push_interleaved_f32(&samples);
        let frames = acc.finish();
        assert_eq!(frames.len(), 1);
        assert!((frames[0].peak_db - 0.0).abs() < 1e-4, "peak saw channel A");
        // RMS over 882 samples half of which are 1.0 → sqrt(0.5).
        assert!((frames[0].rms_db - amplitude_to_db(0.5_f32.sqrt())).abs() < 1e-3);
    }

    #[test]
    fn percentile10_on_known_distribution() {
        // 0..=9 → 10th percentile sits between index 0 and 1 (rank 0.9).
        let values: Vec<f32> = (0..=9).map(|v| v as f32).collect();
        let p10 = percentile10(&values).unwrap();
        assert!((p10 - 0.9).abs() < 1e-6, "got {p10}");

        // Uniform distribution of 100 values 0..99 → p10 ≈ 9.9.
        let many: Vec<f32> = (0..100).map(|v| v as f32).collect();
        let p10 = percentile10(&many).unwrap();
        assert!((p10 - 9.9).abs() < 1e-4, "got {p10}");
    }

    #[test]
    fn percentile_is_none_for_empty_and_exact_on_single() {
        assert!(percentile10(&[]).is_none());
        assert_eq!(percentile10(&[42.0]), Some(42.0));
    }

    #[test]
    fn suggested_threshold_clamps_to_valid_range() {
        assert!((suggested_threshold(-52.0) - (-40.0)).abs() < 1e-6);
        assert_eq!(suggested_threshold(-100.0), -80.0, "clamped low");
        assert_eq!(suggested_threshold(0.0), -10.0, "clamped high");
    }

    #[test]
    fn validate_sample_rate_rejects_zero() {
        assert!(validate_sample_rate(44_100).is_ok());
        assert!(matches!(
            validate_sample_rate(0),
            Err(ApiError::CorruptFile)
        ));
    }
}
