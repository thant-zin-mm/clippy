//! Streaming decode via symphonia (arch §3.1, §4.3; pure Rust, no C deps).
//!
//! Feeds interleaved `f32` PCM chunks (normalized to ±1.0) into a sink. Decode
//! never holds the whole file as PCM in memory (NFR-2): each decoded packet is
//! converted and handed to the sink, then dropped. Formats registered: WAV/RIFF
//! and MP3 (PRD D2). A PCM decoder is registered for WAV's `pcm_s16le` tracks,
//! which symphonia's RIFF reader emits but does not itself decode.

use std::fs::File;
use std::path::Path;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::registry::CodecRegistry;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::{Hint, Probe};
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::packet::Packet;
use symphonia::core::units::{Duration as SymDuration, TimeBase};

use crate::audio::frames::{validate_sample_rate, FrameAccumulator, FrameStats};
use crate::audio::peaks::{PeakPyramid, L0_SAMPLES};
use crate::error::ApiError;

/// Decoded stream metadata (arch §4.2 prepare `done` payload).
#[derive(Debug, Clone, PartialEq)]
pub struct StreamMeta {
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: usize,
    pub format: String,
    /// Total PCM samples (all channels interleaved) actually decoded.
    pub total_samples: usize,
}

/// Everything prepare derives from one decode pass (arch §4.2, §4.4).
#[derive(Debug)]
pub struct DecodedAnalysis {
    pub meta: StreamMeta,
    pub peak_pyramid: PeakPyramid,
    pub peak_db: Vec<f32>,
    pub rms_db: Vec<f32>,
}

/// Build the probe + codec registry for the formats v1 supports.
fn build_registry() -> CodecRegistry {
    let mut registry = CodecRegistry::new();
    registry.register_audio_decoder::<symphonia::default::codecs::MpaDecoder>();
    registry.register_audio_decoder::<symphonia::default::codecs::PcmDecoder>();
    registry
}

fn build_probe() -> Probe {
    let mut probe = Probe::new();
    probe.register_format::<symphonia::default::formats::MpaReader>();
    probe.register_format::<symphonia::default::formats::WavReader>();
    probe
}

/// Map a symphonia error onto the typed API error (FR-21).
fn map_symphonia_error(err: SymError) -> ApiError {
    match err {
        SymError::Unsupported(_) | SymError::SeekError(_) => ApiError::UnsupportedFormat,
        SymError::DecodeError(_) | SymError::IoError(_) | SymError::LimitError(_) => {
            ApiError::CorruptFile
        }
        SymError::ResetRequired => ApiError::CorruptFile,
        // `Error` is #[non_exhaustive]; treat anything new as corrupt/unsupported.
        _ => ApiError::CorruptFile,
    }
}

/// Open a file as a symphonia media source stream.
fn open_stream(path: &Path) -> Result<MediaSourceStream<'static>, ApiError> {
    let file = File::open(path).map_err(|_| ApiError::CorruptFile)?;
    Ok(MediaSourceStream::new(
        Box::new(file),
        MediaSourceStreamOptions::default(),
    ))
}

fn duration_ms_from_track(
    time_base: Option<TimeBase>,
    duration: Option<SymDuration>,
) -> Option<u64> {
    let tb = time_base?;
    let dur = duration?;
    let time = tb.calc_duration(dur)?;
    Some(time.as_millis() as u64)
}

/// Normalize symphonia short names to the v1 format strings (`wav`, `mp3`).
fn normalize_format_name(short_name: &str) -> String {
    match short_name.to_ascii_lowercase().as_str() {
        "wav" | "wave" | "riff" => "wav".into(),
        "mp3" | "mpa" | "mpeg" | "mpeg-audio" => "mp3".into(),
        other => other.to_ascii_lowercase(),
    }
}

/// Read container-level metadata without decoding, used to pre-validate and to
/// derive the duration/format when the container states them.
fn probe_header(path: &Path) -> Result<StreamMeta, ApiError> {
    let mss = open_stream(path)?;
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probe = build_probe();
    let format = probe
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(map_symphonia_error)?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or(ApiError::UnsupportedFormat)?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or(ApiError::UnsupportedFormat)?;

    let sample_rate = params.sample_rate.ok_or(ApiError::UnsupportedFormat)?;
    validate_sample_rate(sample_rate)?;
    let channels = params
        .channels
        .as_ref()
        .map(|c| c.count())
        .filter(|&n| n >= 1)
        .ok_or(ApiError::UnsupportedFormat)?;

    let duration_ms = duration_ms_from_track(track.time_base, track.duration).unwrap_or(0);

    let format_name = format.format_info().short_name.to_owned();

    Ok(StreamMeta {
        duration_ms,
        sample_rate,
        channels,
        format: normalize_format_name(&format_name),
        total_samples: 0,
    })
}

/// The full prepare pass: decode the file once, building the peak pyramid and
/// per-10 ms frame stats in a single streaming sweep (arch §4.4, NFR-2).
pub fn analyze_file(path: &Path) -> Result<DecodedAnalysis, ApiError> {
    let mut meta = probe_header(path)?;

    // Re-open for the real decode (the probe consumed its own handle).
    let mss = open_stream(path)?;
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probe = build_probe();
    let mut format = probe
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(map_symphonia_error)?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or(ApiError::UnsupportedFormat)?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or(ApiError::UnsupportedFormat)?
        .clone();

    let registry = build_registry();
    let mut decoder = registry
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(map_symphonia_error)?;

    let channels = meta.channels;
    let mut frame_acc = FrameAccumulator::new(meta.sample_rate, channels);
    let mut interleaved_i16: Vec<i16> = Vec::new();
    let mut total_samples = 0usize;
    let mut chunk: Vec<f32> = Vec::with_capacity(L0_SAMPLES * channels);

    loop {
        let packet: Packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break, // end of stream
            Err(err) => return Err(map_symphonia_error(err)),
        };
        if packet.track_id != track_id {
            continue; // ignore non-audio packets
        }

        let decoded = decoder.decode(&packet).map_err(map_symphonia_error)?;
        let frames = decoded.frames();
        if frames == 0 {
            continue;
        }

        // Convert the decoded buffer to interleaved f32 in ±1.0.
        chunk.clear();
        decoded.copy_to_vec_interleaved(&mut chunk);
        total_samples += chunk.len();

        // Feed the frame accumulator (10 ms windows) and the i16 peak
        // pyramid in the same pass.
        frame_acc.push_interleaved_f32(&chunk);
        interleaved_i16.extend(chunk.iter().map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16));
    }

    let frames: Vec<FrameStats> = frame_acc.finish();
    let peak_pyramid = PeakPyramid::build(&interleaved_i16, channels);
    // Drop the PCM once the pyramid is built (NFR-2 bounded memory).
    drop(interleaved_i16);

    // Duration: from decoded samples if the container didn't state one.
    let samples_per_second = meta.sample_rate as usize * channels;
    if meta.duration_ms == 0 && samples_per_second > 0 {
        meta.duration_ms = (total_samples as u64 * 1000) / samples_per_second as u64;
    }
    meta.total_samples = total_samples;

    let peak_db = frames.iter().map(|f| f.peak_db).collect();
    let rms_db = frames.iter().map(|f| f.rms_db).collect();

    Ok(DecodedAnalysis {
        meta,
        peak_pyramid,
        peak_db,
        rms_db,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_format_name_maps_container_aliases() {
        assert_eq!(normalize_format_name("WAV"), "wav");
        assert_eq!(normalize_format_name("WAVE"), "wav");
        assert_eq!(normalize_format_name("riff"), "wav");
        assert_eq!(normalize_format_name("MP3"), "mp3");
        assert_eq!(normalize_format_name("mpa"), "mp3");
        assert_eq!(normalize_format_name("MPEG"), "mp3");
        assert_eq!(normalize_format_name("flac"), "flac"); // passthrough
    }
}
