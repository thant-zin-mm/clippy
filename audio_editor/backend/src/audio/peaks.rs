//! Multi-resolution peak pyramid (arch §4.4, D30).
//!
//! Level 0 stores min/max `i16` per `L0_SAMPLES` consecutive samples; each
//! higher level aggregates 4 level-`n` pairs into 1. A windowed query picks
//! the coarsest level whose resolution is still at least as fine as the
//! requested column count, then downsamples to exactly `columns` min/max pairs.
//!
//! The pyramid is built in one streaming pass during prepare, never fully
//! resident as f32 PCM (NFR-2: bounded memory).

use crate::error::ApiError;

/// Samples per level-0 bucket (arch §4.4: L0 = 512 samples).
pub const L0_SAMPLES: usize = 512;
/// Aggregation factor between consecutive levels (arch §4.4: ×4).
pub const LEVEL_FACTOR: usize = 4;
/// Maximum pyramid levels. L4 = 512·4⁴ ≈ 131 072 samples per bucket; for a
/// 4 h file that is ≈ 4.8 k pairs, covering the 4096-column cap (arch §4.4).
pub const MAX_LEVELS: usize = 5;
/// Hard cap on requested columns (arch §6, D30).
pub const MAX_COLUMNS: usize = 4096;

/// One min/max pair at some pyramid level. Stored little-endian `i16` on the
/// wire (D30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinMax {
    pub min: i16,
    pub max: i16,
}

impl MinMax {
    #[inline]
    fn from_sample(sample: i16) -> Self {
        Self {
            min: sample,
            max: sample,
        }
    }

    #[inline]
    fn merge(&mut self, other: Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    #[inline]
    fn merge_scalar(&mut self, sample: i16) {
        self.min = self.min.min(sample);
        self.max = self.max.max(sample);
    }
}

/// Multi-resolution peak pyramid for one source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeakPyramid {
    /// `levels[0]` has one pair per `L0_SAMPLES` samples; `levels[n+1]` has
    /// one pair per `LEVEL_FACTOR` pairs of `levels[n]`.
    levels: Vec<Vec<MinMax>>,
}

impl PeakPyramid {
    /// Build a pyramid from interleaved `i16` PCM.
    ///
    /// `channels` is used only to know when a frame is complete; the pyramid
    /// stores per-sample peaks across all channels (mono view), which is what
    /// the waveform draws.
    pub fn build(interleaved: &[i16], channels: usize) -> Self {
        debug_assert!(channels >= 1, "at least one channel");
        let mut levels: Vec<Vec<MinMax>> = Vec::new();

        // Level 0: min/max over each L0_SAMPLES chunk of samples.
        let mut level0 = Vec::new();
        for chunk in interleaved.chunks(L0_SAMPLES) {
            let mut acc = MinMax::from_sample(chunk[0]);
            for &sample in &chunk[1..] {
                acc.merge_scalar(sample);
            }
            level0.push(acc);
        }
        levels.push(level0);

        // Higher levels: 4 pairs → 1.
        while levels.len() < MAX_LEVELS {
            let prev = levels.last().expect("at least one level");
            if prev.len() < LEVEL_FACTOR {
                break; // nothing left to aggregate
            }
            let mut next = Vec::with_capacity(prev.len() / LEVEL_FACTOR);
            for group in prev.chunks(LEVEL_FACTOR) {
                let mut acc = group[0];
                for &pair in &group[1..] {
                    acc.merge(pair);
                }
                next.push(acc);
            }
            levels.push(next);
        }

        Self { levels }
    }

    /// Number of pyramid levels (1..=MAX_LEVELS; 0 only for empty input).
    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    /// Total samples covered by level 0 (rounded up to a full bucket).
    pub fn total_samples(&self) -> usize {
        self.levels.first().map_or(0, |l0| l0.len() * L0_SAMPLES)
    }

    /// Level chosen for a query requesting `columns` buckets over
    /// `[start_sample, end_sample)`: the coarsest level whose bucket span is
    /// still `<=` the ideal bucket span, so downsampling stays exact.
    fn select_level(&self, start_sample: usize, end_sample: usize, columns: usize) -> usize {
        let span = end_sample.saturating_sub(start_sample);
        if span == 0 || columns == 0 {
            return 0;
        }
        let ideal = span as f64 / columns as f64;
        let mut chosen = 0;
        for (idx, _) in self.levels.iter().enumerate() {
            let bucket = bucket_samples(idx);
            if (bucket as f64) <= ideal {
                chosen = idx;
            } else {
                break;
            }
        }
        chosen
    }

    /// Min/max pairs for `[start_sample, end_sample)` downsampled to exactly
    /// `columns` pairs.
    ///
    /// Each output column owns the exact sample range it would draw
    /// (`[start + col·span/columns, start + (col+1)·span/columns)`), merged
    /// from every pyramid bucket that overlaps that range. With the coarsest
    /// level whose bucket size fits the ideal column width, a column can span
    /// a bucket boundary: its result is then a *superset* of the exact-range
    /// data (bucket-quantized), never a subset — so no signal is missed.
    ///
    /// Returns `Err(E_INVALID_PARAMETER)` on out-of-range/empty input or
    /// `columns > MAX_COLUMNS`; `columns == 0` yields an empty vector.
    pub fn window(
        &self,
        start_sample: usize,
        end_sample: usize,
        columns: usize,
    ) -> Result<Vec<MinMax>, ApiError> {
        if columns > MAX_COLUMNS {
            return Err(ApiError::InvalidParameter(format!(
                "columns must be <= {MAX_COLUMNS}"
            )));
        }
        if end_sample <= start_sample {
            return Err(ApiError::InvalidParameter(
                "end_sample must be greater than start_sample".into(),
            ));
        }
        if columns == 0 {
            return Ok(Vec::new());
        }

        let level = self.select_level(start_sample, end_sample, columns);
        let bucket = bucket_samples(level);
        let pairs = &self.levels[level];
        if pairs.is_empty() {
            return Ok(vec![MinMax { min: 0, max: 0 }; columns]);
        }
        let max_idx = pairs.len() - 1;
        let span = end_sample - start_sample;

        let mut out = Vec::with_capacity(columns);
        for col in 0..columns {
            // Exact column range (same arithmetic as a full-resolution scan).
            let from = start_sample + (col * span) / columns;
            let to_excl = (start_sample + ((col + 1) * span) / columns)
                .max(from + 1)
                .min(end_sample);
            let to = to_excl - 1;

            // Merge every bucket overlapping [from, to].
            let first_idx = (from / bucket).min(max_idx);
            let last_idx = (to / bucket).min(max_idx);
            let mut acc = pairs[first_idx];
            for pair in &pairs[first_idx..=last_idx] {
                acc.merge(*pair);
            }
            out.push(acc);
        }
        Ok(out)
    }

    /// Serialize pairs to little-endian `i16` bytes (min then max per pair),
    /// the wire format for `GET /api/sources/{id}/peaks` (D30).
    pub fn encode_pairs(pairs: &[MinMax]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(pairs.len() * 4);
        for pair in pairs {
            bytes.extend_from_slice(&pair.min.to_le_bytes());
            bytes.extend_from_slice(&pair.max.to_le_bytes());
        }
        bytes
    }
}

/// Samples per bucket at a given pyramid level.
#[inline]
fn bucket_samples(level: usize) -> usize {
    L0_SAMPLES * LEVEL_FACTOR.pow(level as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic pseudo-random i16 stream (no external RNG dep).
    fn pseudo_random_i16(len: usize, seed: u64) -> Vec<i16> {
        let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (0..len)
            .map(|_| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let word = (state >> 33) as u32;
                (word >> 16) as i16
            })
            .collect()
    }

    /// Brute-force min/max over each column's exact sample range — the
    /// full-resolution reference for `window` (same column arithmetic).
    fn brute_force(samples: &[i16], start: usize, end: usize, columns: usize) -> Vec<MinMax> {
        let mut out = Vec::with_capacity(columns);
        let span = end - start;
        for col in 0..columns {
            let from = start + (col * span) / columns;
            let to_excl = (start + ((col + 1) * span) / columns)
                .max(from + 1)
                .min(end);
            let to = to_excl - 1;
            let mut acc = MinMax::from_sample(samples[from]);
            for &s in &samples[from..=to] {
                acc.merge_scalar(s);
            }
            out.push(acc);
        }
        out
    }

    #[test]
    fn level0_buckets_are_512_samples() {
        let samples = pseudo_random_i16(L0_SAMPLES * 3, 7);
        let pyr = PeakPyramid::build(&samples, 1);
        assert_eq!(pyr.num_levels(), 1, "3 partial buckets stay at level 0");
        assert_eq!(pyr.levels[0].len(), 3);
    }

    #[test]
    fn levels_grow_while_input_is_long_enough() {
        // 4^4 = 256 level-0 buckets → exactly MAX_LEVELS levels.
        let len = L0_SAMPLES * LEVEL_FACTOR.pow(4);
        let samples = pseudo_random_i16(len, 11);
        let pyr = PeakPyramid::build(&samples, 1);
        assert_eq!(pyr.num_levels(), MAX_LEVELS);
        assert_eq!(pyr.total_samples(), len);
        assert_eq!(pyr.levels[4].len(), 1, "top level collapses to one pair");
    }

    /// Across shapes/zooms, every window column must *contain* the exact
    /// full-resolution data of its own sample range (the pyramid may add
    /// bucket-quantized bleed at column edges, but never misses signal).
    #[test]
    fn window_contains_brute_force_across_shapes() {
        let len = L0_SAMPLES * LEVEL_FACTOR.pow(4) + 1234;
        let samples = pseudo_random_i16(len, 2024);
        let pyr = PeakPyramid::build(&samples, 1);

        let cases: &[(usize, usize, usize)] = &[
            (0, len, 1),
            (0, len, 7),
            (0, len, 100),
            (0, len, 4096),
            (1000, 5000, 50),
            (L0_SAMPLES * 10, L0_SAMPLES * 10 + 4096, 128),
            (len - 1, len, 1),
            (len - 1, len, 4),
            (len - 1, len, 9),
        ];
        for &(start, end, columns) in cases {
            let got = pyr.window(start, end, columns).expect("valid window");
            let want = brute_force(&samples, start, end, columns);
            assert_eq!(
                got.len(),
                columns,
                "columns mismatch for window {start}..{end} cols={columns}"
            );
            for col in 0..columns {
                let (g, w) = (got[col], want[col]);
                assert!(
                    g.min <= w.min && g.max >= w.max,
                    "window {start}..{end} cols={columns} col {col}: \
                     {g:?} must contain exact {w:?}"
                );
            }
        }
    }

    /// When column boundaries land on bucket boundaries, the pyramid answer is
    /// bit-exact with a full-resolution scan.
    #[test]
    fn window_is_exact_on_bucket_aligned_ranges() {
        // 131072 samples → levels [256, 64, 16, 4, 1] with buckets
        // 512 / 2048 / 8192 / 32768 / 131072; 1, 4, and 16 columns over the
        // full file split exactly on those bucket edges.
        let len = L0_SAMPLES * LEVEL_FACTOR.pow(4);
        let samples = pseudo_random_i16(len, 31);
        let pyr = PeakPyramid::build(&samples, 1);
        for columns in [1usize, 4, 16] {
            let got = pyr.window(0, len, columns).expect("valid window");
            let want = brute_force(&samples, 0, len, columns);
            assert_eq!(
                got, want,
                "aligned window must equal brute force (columns={columns})"
            );
        }
    }

    #[test]
    fn full_file_single_column_equals_global_minmax() {
        let samples = pseudo_random_i16(L0_SAMPLES * 20, 5);
        let pyr = PeakPyramid::build(&samples, 1);
        let got = pyr.window(0, samples.len(), 1).unwrap();
        let global = brute_force(&samples, 0, samples.len(), 1);
        assert_eq!(got, global);
        // And it really is the global min/max.
        assert_eq!(got[0].min, *samples.iter().min().unwrap());
        assert_eq!(got[0].max, *samples.iter().max().unwrap());
    }

    #[test]
    fn rejects_out_of_range_and_oversized_columns() {
        let samples = pseudo_random_i16(L0_SAMPLES * 4, 3);
        let pyr = PeakPyramid::build(&samples, 1);
        assert!(matches!(
            pyr.window(0, samples.len(), MAX_COLUMNS + 1),
            Err(ApiError::InvalidParameter(_))
        ));
        assert!(matches!(
            pyr.window(10, 10, 10),
            Err(ApiError::InvalidParameter(_))
        ));
        assert!(matches!(
            pyr.window(5, 3, 10),
            Err(ApiError::InvalidParameter(_))
        ));
    }

    #[test]
    fn zero_columns_is_empty_ok() {
        let samples = pseudo_random_i16(L0_SAMPLES * 2, 9);
        let pyr = PeakPyramid::build(&samples, 1);
        assert!(pyr.window(0, samples.len(), 0).unwrap().is_empty());
    }

    #[test]
    fn encode_pairs_is_little_endian_min_then_max() {
        let pairs = [MinMax { min: -2, max: 3 }];
        let bytes = PeakPyramid::encode_pairs(&pairs);
        assert_eq!(bytes, vec![0xFE, 0xFF, 0x03, 0x00]);
        assert_eq!(bytes.len(), pairs.len() * 4);
    }

    #[test]
    fn multichannel_input_is_treated_as_interleaved_samples() {
        // Two channels interleaved; peaks are per-sample regardless of layout.
        let samples: Vec<i16> = vec![100, -100, 200, -200, 50, -50, 0, 0];
        let pyr = PeakPyramid::build(&samples, 2);
        let got = pyr.window(0, samples.len(), 1).unwrap();
        assert_eq!(got[0].min, -200);
        assert_eq!(got[0].max, 200);
    }
}
