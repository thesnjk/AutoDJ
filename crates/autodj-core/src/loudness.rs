//! EBU R 128 / ITU-R BS.1770 integrated loudness helpers.
//!
//! Offline path only — analyzers allocate; do not call from a real-time audio thread.

use autodj_io::AudioBuffer;
use ebur128::{EbuR128, Mode};
use thiserror::Error;

/// Default DJ / podcast continuous-mix target (streaming-style).
///
/// Broadcast delivery can pass `--lufs -23` for classic EBU R 128 programme loudness.
pub const DEFAULT_TARGET_LUFS: f64 = -14.0;

/// Classic EBU R 128 programme loudness target.
pub const BROADCAST_TARGET_LUFS: f64 = -23.0;

/// Pass window for automated loudness gates (LU).
pub const LUFS_TOLERANCE_LU: f64 = 2.0;

/// True-peak ceiling applied after loudness gain (linear full-scale).
const TRUE_PEAK_CEILING: f32 = 0.99;

/// Errors from loudness measurement or normalization.
#[derive(Debug, Error)]
pub enum LoudnessError {
    /// Underlying ebur128 failure.
    #[error("ebur128: {0}")]
    Ebu(#[from] ebur128::Error),
    /// Buffer has unsupported channel layout.
    #[error("unsupported channel count: {0}")]
    Channels(u16),
    /// Integrated loudness could not be computed (too short / silent).
    #[error("integrated loudness unavailable (audio too short or silent)")]
    Unavailable,
}

/// Measured loudness summary for a buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessReport {
    /// Integrated (gated) loudness in LUFS.
    pub integrated_lufs: f64,
    /// Maximum true-peak across channels (linear full-scale).
    pub true_peak: f64,
}

/// Measure integrated loudness and true peak for an interleaved buffer.
pub fn measure_loudness(buffer: &AudioBuffer) -> Result<LoudnessReport, LoudnessError> {
    if buffer.channels == 0 || buffer.channels > 2 {
        return Err(LoudnessError::Channels(buffer.channels));
    }
    if buffer.samples.is_empty() {
        return Err(LoudnessError::Unavailable);
    }

    let channels = buffer.channels as u32;
    let mut ebu = EbuR128::new(
        channels,
        buffer.sample_rate,
        Mode::I | Mode::TRUE_PEAK | Mode::HISTOGRAM,
    )?;

    // Feed ~1s chunks; ebur128 accepts arbitrary frame counts.
    let frame_chunk = buffer.sample_rate as usize;
    let ch = channels as usize;
    for frame_block in buffer.samples.chunks(frame_chunk * ch) {
        ebu.add_frames_f32(frame_block)?;
    }

    let integrated = ebu.loudness_global().map_err(|_| LoudnessError::Unavailable)?;
    if !integrated.is_finite() {
        return Err(LoudnessError::Unavailable);
    }

    let mut true_peak = 0.0f64;
    for c in 0..channels {
        true_peak = true_peak.max(ebu.true_peak(c)?);
    }

    Ok(LoudnessReport {
        integrated_lufs: integrated,
        true_peak,
    })
}

/// Scale `buffer` so integrated loudness approaches `target_lufs`, then peak-limit.
///
/// Returns the post-normalization measurement. Gain is capped so true-peak stays
/// ≤ [`TRUE_PEAK_CEILING`] after scaling.
pub fn normalize_loudness(
    buffer: &mut AudioBuffer,
    target_lufs: f64,
) -> Result<LoudnessReport, LoudnessError> {
    let before = measure_loudness(buffer)?;
    let gain_db = target_lufs - before.integrated_lufs;
    let mut gain = 10.0f64.powf(gain_db / 20.0) as f32;

    let peak_after = before.true_peak as f32 * gain;
    if peak_after > TRUE_PEAK_CEILING && before.true_peak > 1e-9 {
        gain = TRUE_PEAK_CEILING / before.true_peak as f32;
    }

    for s in buffer.samples.iter_mut() {
        *s = (*s * gain).clamp(-1.0, 1.0);
    }

    // Safety peak limit if true-peak estimate was optimistic.
    let mut peak = 0.0f32;
    for &s in buffer.samples.iter() {
        peak = peak.max(s.abs());
    }
    if peak > TRUE_PEAK_CEILING {
        let g = TRUE_PEAK_CEILING / peak;
        for s in buffer.samples.iter_mut() {
            *s *= g;
        }
    }

    measure_loudness(buffer)
}

/// Whether `measured` is within [`LUFS_TOLERANCE_LU`] of `target_lufs`.
pub fn within_lufs_tolerance(measured: f64, target_lufs: f64) -> bool {
    (measured - target_lufs).abs() <= LUFS_TOLERANCE_LU
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stereo_tone(sr: u32, secs: f32, freq: f32, amp: f32) -> AudioBuffer {
        let n = (sr as f32 * secs) as usize;
        let mut samples = Vec::with_capacity(n * 2);
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let s = (2.0 * std::f32::consts::PI * freq * t).sin() * amp;
            samples.push(s);
            samples.push(s);
        }
        AudioBuffer {
            sample_rate: sr,
            channels: 2,
            samples,
        }
    }

    #[test]
    fn measures_finite_integrated_loudness() {
        let buf = stereo_tone(48_000, 3.0, 1000.0, 0.2);
        let report = measure_loudness(&buf).expect("measure");
        assert!(report.integrated_lufs.is_finite());
        assert!(report.true_peak > 0.0);
    }

    #[test]
    fn normalizes_within_tolerance() {
        let mut buf = stereo_tone(48_000, 4.0, 1000.0, 0.05);
        let target = DEFAULT_TARGET_LUFS;
        let after = normalize_loudness(&mut buf, target).expect("normalize");
        assert!(
            within_lufs_tolerance(after.integrated_lufs, target),
            "got {:.2} LUFS, target {:.2}",
            after.integrated_lufs,
            target
        );
        let peak = buf.samples.iter().copied().map(f32::abs).fold(0.0, f32::max);
        assert!(peak <= 1.0);
    }
}
