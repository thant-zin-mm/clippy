//! Prepare pass: derive the `done` payload + cached analysis (arch §4.2, §4.3).
//!
//! The decode pass produces the peak pyramid and per-frame loudness arrays;
//! here we compute the noise floor (10th percentile of `rms_db`) and the
//! suggested detection threshold (noise floor + 12 dB, clamped to [−80, −10]).

use std::path::Path;

use serde::Serialize;

use crate::audio::decode::{analyze_file, DecodedAnalysis, StreamMeta};
use crate::audio::frames::{percentile10, suggested_threshold};
use crate::audio::peaks::PeakPyramid;
use crate::error::ApiError;

/// Result of a successful prepare (arch §4.2 `done` payload).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PrepareResult {
    pub source_id: String,
    pub filename: String,
    pub format: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: usize,
    pub noise_floor_db: f32,
    pub suggested_threshold_db: f32,
}

/// Cached analysis for one prepared source, kept server-side (arch §4.2).
#[derive(Debug)]
pub struct PreparedSource {
    pub meta: StreamMeta,
    pub peak_pyramid: PeakPyramid,
    pub peak_db: Vec<f32>,
    pub rms_db: Vec<f32>,
    pub noise_floor_db: f32,
    pub suggested_threshold_db: f32,
}

/// Run the full prepare analysis on a temp file and package the result.
pub fn prepare_file(
    source_id: &str,
    filename: &str,
    path: &Path,
) -> Result<(PrepareResult, PreparedSource), ApiError> {
    let DecodedAnalysis {
        meta,
        peak_pyramid,
        peak_db,
        rms_db,
    } = analyze_file(path)?;

    let noise_floor_db = percentile10(&rms_db).unwrap_or(crate::audio::frames::DB_FLOOR);
    let suggested = suggested_threshold(noise_floor_db);

    let result = PrepareResult {
        source_id: source_id.to_owned(),
        filename: filename.to_owned(),
        format: meta.format.clone(),
        duration_ms: meta.duration_ms,
        sample_rate: meta.sample_rate,
        channels: meta.channels,
        noise_floor_db,
        suggested_threshold_db: suggested,
    };

    let prepared = PreparedSource {
        meta,
        peak_pyramid,
        peak_db,
        rms_db,
        noise_floor_db,
        suggested_threshold_db: suggested,
    };

    Ok((result, prepared))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggested_threshold_is_noise_floor_plus_12_db() {
        // prepare_file is covered by the integration suite on real fixtures;
        // here we lock the suggestion math that the payload depends on.
        let floor: f32 = -52.3;
        let expected = (floor + 12.0).clamp(-80.0, -10.0);
        assert!((suggested_threshold(floor) - expected).abs() < 1e-6);
        assert_eq!(suggested_threshold(-95.0), -80.0); // clamped low
        assert_eq!(suggested_threshold(0.0), -10.0); // clamped high
    }
}
