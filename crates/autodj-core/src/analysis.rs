//! Offline track feature extraction.

use std::path::{Path, PathBuf};

use autodj_io::{read_wav, AudioBuffer};
use serde::{Deserialize, Serialize};

use crate::camelot::{from_pitch_class, CamelotKey};

/// Predicted transition style between two tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionClass {
    /// Linear amplitude crossfade.
    LinearCrossfade,
    /// Bass/mid/high EQ swap.
    EqSwap,
    /// High-pass / low-pass sweep.
    FilterSweep,
    /// Echo / delay out on outgoing track.
    EchoOut,
    /// Hard cut on a downbeat.
    HardCut,
}

/// Analyzed features for one track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackFeatures {
    /// Source path when known.
    pub path: Option<PathBuf>,
    /// Estimated BPM.
    pub bpm: f32,
    /// Camelot key estimate.
    pub key: CamelotKey,
    /// Integrated loudness proxy (RMS dBFS-ish).
    pub loudness_db: f32,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Sample rate used for analysis.
    pub sample_rate: u32,
    /// Coarse section boundaries in seconds: intro end, drop start, outro start.
    pub sections: SectionMarks,
    /// Mean spectral centroid proxy (Hz).
    pub spectral_centroid_hz: f32,
}

/// Coarse structural marks used for cueing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionMarks {
    /// End of intro region (seconds).
    pub intro_end: f64,
    /// Approximate drop / hook start (seconds).
    pub drop_start: f64,
    /// Start of outro region (seconds).
    pub outro_start: f64,
}

/// Analyze a WAV path.
pub fn analyze_path(path: impl AsRef<Path>) -> anyhow::Result<TrackFeatures> {
    let path = path.as_ref();
    let buf = read_wav(path)?;
    let mut feats = analyze_buffer(&buf)?;
    feats.path = Some(path.to_path_buf());
    Ok(feats)
}

/// Analyze an in-memory buffer.
pub fn analyze_buffer(buf: &AudioBuffer) -> anyhow::Result<TrackFeatures> {
    let mono = buf.to_mono();
    analyze_mono(&mono, buf.sample_rate)
}

/// Analyze mono PCM.
pub fn analyze_mono(samples: &[f32], sample_rate: u32) -> anyhow::Result<TrackFeatures> {
    anyhow::ensure!(!samples.is_empty(), "empty audio");
    let duration_secs = samples.len() as f64 / sample_rate as f64;
    let bpm = estimate_bpm(samples, sample_rate);
    let (pc, minor) = estimate_key(samples, sample_rate);
    let key = from_pitch_class(pc, minor);
    let loudness_db = rms_db(samples);
    let spectral_centroid_hz = spectral_centroid(samples, sample_rate);
    let sections = estimate_sections(samples, sample_rate, duration_secs);
    Ok(TrackFeatures {
        path: None,
        bpm,
        key,
        loudness_db,
        duration_secs,
        sample_rate,
        sections,
        spectral_centroid_hz,
    })
}

/// Estimate coarse intro / drop / outro marks from a windowed RMS envelope.
fn estimate_sections(samples: &[f32], sample_rate: u32, duration_secs: f64) -> SectionMarks {
    let fallback = SectionMarks {
        intro_end: (duration_secs * 0.15).clamp(0.0, duration_secs),
        drop_start: (duration_secs * 0.35).clamp(0.0, duration_secs),
        outro_start: (duration_secs * 0.75).clamp(0.0, duration_secs),
    };

    let hop = ((sample_rate as f64) * 0.5).round().max(1.0) as usize;
    if samples.len() < hop * 4 {
        return fallback;
    }

    let mut env: Vec<f32> = Vec::new();
    for chunk in samples.chunks(hop) {
        let sum: f64 = chunk.iter().map(|s| (*s as f64).powi(2)).sum();
        env.push((sum / chunk.len() as f64).sqrt() as f32);
    }
    if env.is_empty() {
        return fallback;
    }

    let mut sorted = env.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2];
    let threshold = (median * 1.15).max(1e-6);
    let hop_secs = hop as f64 / sample_rate as f64;

    // Intro end: first window past median energy that holds for 2 hops.
    let mut intro_end = fallback.intro_end;
    for i in 0..env.len().saturating_sub(1) {
        if env[i] >= threshold && env[i + 1] >= threshold {
            intro_end = ((i + 1) as f64 * hop_secs).clamp(0.0, duration_secs);
            break;
        }
    }

    // Outro start: last sustained drop below median in the final third.
    let final_third = (env.len() * 2) / 3;
    let mut outro_start = fallback.outro_start;
    let low = median * 0.85;
    for i in (final_third..env.len().saturating_sub(1)).rev() {
        if env[i] <= low && env[i + 1] <= low {
            outro_start = (i as f64 * hop_secs).clamp(0.0, duration_secs);
            break;
        }
    }

    // Drop: peak energy between intro and outro.
    let start_i = ((intro_end / hop_secs).floor() as usize).min(env.len().saturating_sub(1));
    let end_i = ((outro_start / hop_secs).floor() as usize)
        .max(start_i)
        .min(env.len().saturating_sub(1));
    let mut peak_i = start_i;
    let mut peak_v = env[start_i];
    for (i, &v) in env.iter().enumerate().take(end_i + 1).skip(start_i) {
        if v > peak_v {
            peak_v = v;
            peak_i = i;
        }
    }
    let mut drop_start = (peak_i as f64 * hop_secs).clamp(0.0, duration_secs);

    // Keep ordering sensible.
    if intro_end > outro_start {
        return fallback;
    }
    if drop_start < intro_end {
        drop_start = intro_end;
    }
    if drop_start > outro_start {
        drop_start = (intro_end + outro_start) * 0.5;
    }

    SectionMarks {
        intro_end,
        drop_start,
        outro_start,
    }
}

fn rms_db(samples: &[f32]) -> f32 {
    let sum: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
    let rms = (sum / samples.len() as f64).sqrt().max(1e-9);
    (20.0 * rms.log10()) as f32
}

fn estimate_bpm(samples: &[f32], sample_rate: u32) -> f32 {
    // Onset strength via half-wave rectified spectral flux proxy (time-domain energy diff).
    let hop = (sample_rate as usize / 50).max(1); // ~20 ms
    let mut env = Vec::new();
    let mut prev = 0.0f32;
    for chunk in samples.chunks(hop) {
        let e: f32 = chunk.iter().map(|s| s.abs()).sum::<f32>() / chunk.len() as f32;
        env.push((e - prev).max(0.0));
        prev = e;
    }
    if env.len() < 8 {
        return 120.0;
    }
    // Autocorrelation over BPM range 70–180.
    let fps = sample_rate as f32 / hop as f32;
    let mut best_bpm = 120.0f32;
    let mut best_score = f32::MIN;
    let mut bpm = 70.0f32;
    while bpm <= 180.0 {
        let lag = (60.0 * fps / bpm).round() as usize;
        if lag == 0 || lag >= env.len() {
            bpm += 0.5;
            continue;
        }
        let mut score = 0.0f32;
        for i in lag..env.len() {
            score += env[i] * env[i - lag];
        }
        if score > best_score {
            best_score = score;
            best_bpm = bpm;
        }
        bpm += 0.5;
    }
    best_bpm
}

fn estimate_key(samples: &[f32], sample_rate: u32) -> (u8, bool) {
    // Crude chroma via Goertzel-like energy at pitch-class frequencies over one octave.
    let base = 110.0f32; // A2
    let mut chroma = [0.0f64; 12];
    let n = samples.len().min(sample_rate as usize * 4);
    let slice = &samples[..n];
    for (pc, bin) in chroma.iter_mut().enumerate() {
        let freq = base * 2f32.powf(pc as f32 / 12.0);
        let w = 2.0 * std::f32::consts::PI * freq / sample_rate as f32;
        let mut cos = 0.0f64;
        let mut sin = 0.0f64;
        for (i, &s) in slice.iter().enumerate() {
            let t = i as f32;
            cos += s as f64 * (w * t).cos() as f64;
            sin += s as f64 * (w * t).sin() as f64;
        }
        *bin = cos * cos + sin * sin;
    }
    let mut best = 0usize;
    for i in 1..12 {
        if chroma[i] > chroma[best] {
            best = i;
        }
    }
    // Minor if neighbors of peak are stronger on the minor triad side (very rough).
    let major_third = chroma[(best + 4) % 12];
    let minor_third = chroma[(best + 3) % 12];
    let minor = minor_third > major_third;
    (best as u8, minor)
}

fn spectral_centroid(samples: &[f32], sample_rate: u32) -> f32 {
    // Zero-crossing / brightness proxy for MVP (true FFT later).
    if samples.len() < 2 {
        return 0.0;
    }
    let mut zc = 0usize;
    for w in samples.windows(2) {
        if w[0].signum() != w[1].signum() {
            zc += 1;
        }
    }
    let zcr = zc as f32 * sample_rate as f32 / samples.len() as f32;
    (zcr * 0.5).clamp(0.0, sample_rate as f32 / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_synthetic_tone() {
        let sr = 44100u32;
        let bpm = 128.0f32;
        let seconds = 4.0f64;
        let n = (sr as f64 * seconds) as usize;
        let beat = 60.0 / bpm;
        let mut samples = vec![0.0f32; n];
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let phase = (t as f64 % beat as f64) / beat as f64;
            let click = if phase < 0.05 {
                (1.0 - phase / 0.05) as f32 * 0.8
            } else {
                0.0
            };
            samples[i] = 0.2 * (2.0 * std::f32::consts::PI * 220.0 * t).sin() + click;
        }
        let feats = analyze_mono(&samples, sr).unwrap();
        assert!(feats.bpm > 100.0 && feats.bpm < 160.0, "bpm={}", feats.bpm);
        assert!(feats.duration_secs > 3.5);
    }

    #[test]
    fn sections_follow_energy_ramp_and_decay() {
        let sr = 44100u32;
        let seconds = 10.0f64;
        let n = (sr as f64 * seconds) as usize;
        let mut samples = vec![0.0f32; n];
        for i in 0..n {
            let t = i as f64 / sr as f64;
            let amp = if t < 2.0 {
                0.05
            } else if t < 7.0 {
                0.4
            } else {
                0.05
            };
            let tone = (2.0 * std::f32::consts::PI * 220.0 * (i as f32 / sr as f32)).sin();
            samples[i] = tone * amp as f32;
        }
        let feats = analyze_mono(&samples, sr).unwrap();
        assert!(
            feats.sections.intro_end < feats.sections.drop_start,
            "intro={} drop={}",
            feats.sections.intro_end,
            feats.sections.drop_start
        );
        assert!(
            feats.sections.drop_start < feats.sections.outro_start,
            "drop={} outro={}",
            feats.sections.drop_start,
            feats.sections.outro_start
        );
        assert!(
            feats.sections.outro_start >= 6.0,
            "outro_start={}",
            feats.sections.outro_start
        );
    }
}
