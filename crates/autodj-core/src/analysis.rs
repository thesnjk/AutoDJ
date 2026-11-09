//! Offline track feature extraction.
//!
//! # Real-time safety
//! Analysis allocates and scans whole buffers — **offline only**. Do not call
//! [`analyze_mono`] / [`analyze_path`] from an audio callback.

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

/// Confidence and failure-mode flags for an analysis pass.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisQuality {
    /// BPM estimate confidence in `0.0..=1.0`.
    pub bpm_confidence: f32,
    /// Key estimate confidence in `0.0..=1.0`.
    pub key_confidence: f32,
    /// True when the buffer is near-silence or too short for reliable analysis.
    pub sparse_audio: bool,
    /// True when BPM was doubled after autocorr preferred a half-time peak.
    pub bpm_octave_corrected: bool,
}

impl Default for AnalysisQuality {
    fn default() -> Self {
        Self {
            bpm_confidence: 0.0,
            key_confidence: 0.0,
            sparse_audio: false,
            bpm_octave_corrected: false,
        }
    }
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
    /// Beat-grid phase offset in seconds (`[0, beat)`): first downbeat relative to t=0.
    #[serde(default)]
    pub beat_offset_secs: f64,
    /// Confidence / sparse / octave-correction flags.
    #[serde(default)]
    pub quality: AnalysisQuality,
    /// Phrase length in bars (`4`/`8`/`16`). `0` = unset; resolve at mix via estimator.
    #[serde(default)]
    pub bars_per_phrase: u32,
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

/// Analyze a WAV path (offline only).
pub fn analyze_path(path: impl AsRef<Path>) -> anyhow::Result<TrackFeatures> {
    let path = path.as_ref();
    let buf = read_wav(path)?;
    let mut feats = analyze_buffer(&buf)?;
    feats.path = Some(path.to_path_buf());
    Ok(feats)
}

/// Analyze an in-memory buffer (offline only).
pub fn analyze_buffer(buf: &AudioBuffer) -> anyhow::Result<TrackFeatures> {
    let mono = buf.to_mono();
    analyze_mono(&mono, buf.sample_rate)
}

/// Analyze mono PCM (offline only — heap OK).
pub fn analyze_mono(samples: &[f32], sample_rate: u32) -> anyhow::Result<TrackFeatures> {
    anyhow::ensure!(!samples.is_empty(), "empty audio");
    let duration_secs = samples.len() as f64 / sample_rate as f64;
    let loudness_db = rms_db(samples);
    let sparse = is_sparse(samples, duration_secs, loudness_db);

    let (bpm, bpm_confidence, bpm_octave_corrected) = if sparse {
        (120.0, 0.0, false)
    } else {
        estimate_bpm(samples, sample_rate)
    };

    let (pc, minor, key_confidence) = if sparse {
        (0u8, true, 0.0)
    } else {
        estimate_key(samples, sample_rate)
    };
    let key = from_pitch_class(pc, minor);

    let mut bpm_confidence = bpm_confidence;
    let mut key_confidence = key_confidence;
    if duration_secs < 2.0 {
        bpm_confidence *= 0.35;
        key_confidence *= 0.35;
    }

    let spectral_centroid_hz = spectral_centroid(samples, sample_rate);
    let sections = estimate_sections(samples, sample_rate, duration_secs);
    let beat_offset_secs = if sparse {
        0.0
    } else {
        estimate_beat_offset(samples, sample_rate, bpm)
    };
    Ok(TrackFeatures {
        path: None,
        bpm,
        key,
        loudness_db,
        duration_secs,
        sample_rate,
        sections,
        spectral_centroid_hz,
        beat_offset_secs,
        quality: AnalysisQuality {
            bpm_confidence,
            key_confidence,
            sparse_audio: sparse,
            bpm_octave_corrected,
        },
        // Resolved at mix with genre fallback via `estimate_bars_per_phrase`.
        bars_per_phrase: 0,
    })
}

fn is_sparse(samples: &[f32], duration_secs: f64, loudness_db: f32) -> bool {
    if duration_secs < 0.25 {
        return true;
    }
    // Near silence.
    if loudness_db < -55.0 {
        return true;
    }
    let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    peak < 1e-4
}

/// Estimate coarse intro / drop / outro marks from a windowed RMS envelope.
///
/// Falls back to 15% / 35% / 75% of duration when the buffer is too short or
/// ordering cannot be established.
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

    let mut intro_end = fallback.intro_end;
    for i in 0..env.len().saturating_sub(1) {
        if env[i] >= threshold && env[i + 1] >= threshold {
            intro_end = ((i + 1) as f64 * hop_secs).clamp(0.0, duration_secs);
            break;
        }
    }

    let final_third = (env.len() * 2) / 3;
    let mut outro_start = fallback.outro_start;
    let low = median * 0.85;
    for i in (final_third..env.len().saturating_sub(1)).rev() {
        if env[i] <= low && env[i + 1] <= low {
            outro_start = (i as f64 * hop_secs).clamp(0.0, duration_secs);
            break;
        }
    }

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

/// Returns `(bpm, confidence, octave_corrected)`.
fn estimate_bpm(samples: &[f32], sample_rate: u32) -> (f32, f32, bool) {
    let hop = (sample_rate as usize / 50).max(1); // ~20 ms
    let mut env = Vec::new();
    let mut prev = 0.0f32;
    for chunk in samples.chunks(hop) {
        let e: f32 = chunk.iter().map(|s| s.abs()).sum::<f32>() / chunk.len() as f32;
        env.push((e - prev).max(0.0));
        prev = e;
    }
    if env.len() < 8 {
        return (120.0, 0.0, false);
    }
    let fps = sample_rate as f32 / hop as f32;

    let score_at = |bpm: f32| -> f32 {
        let lag = (60.0 * fps / bpm).round() as usize;
        if lag == 0 || lag >= env.len() {
            return f32::MIN / 4.0;
        }
        let mut score = 0.0f32;
        for i in lag..env.len() {
            score += env[i] * env[i - lag];
        }
        score
    };

    let mut best_bpm = 120.0f32;
    let mut best_score = f32::MIN;
    let mut second_score = f32::MIN;
    let mut bpm = 70.0f32;
    while bpm <= 180.0 {
        let score = score_at(bpm);
        if score > best_score {
            second_score = best_score;
            best_score = score;
            best_bpm = bpm;
        } else if score > second_score {
            second_score = score;
        }
        bpm += 0.5;
    }

    // Among {peak, 2×, ½×} pick max (autocorr_score * dance-tempo prior).
    // Fixes half-time peaks (e.g. 70→140) without always forcing doubles on true slow tracks.
    let raw_peak = best_bpm;
    let candidates = [raw_peak, raw_peak * 2.0, raw_peak * 0.5];
    let mut pick = raw_peak;
    let mut pick_score = f32::MIN;
    let mut pick_raw = best_score;
    for &c in &candidates {
        if !(70.0..=180.0).contains(&c) {
            continue;
        }
        let s = score_at(c);
        if !s.is_finite() || s <= 0.0 {
            continue;
        }
        let weighted = s * tempo_prior(c);
        if weighted > pick_score {
            pick_score = weighted;
            pick = c;
            pick_raw = s;
        }
    }
    let octave_corrected = (pick - raw_peak).abs() > 0.25;
    best_bpm = pick;
    if pick_raw > best_score {
        second_score = best_score;
        best_score = pick_raw;
    }

    let confidence = if best_score <= 0.0 || !best_score.is_finite() {
        0.0
    } else if second_score <= 0.0 || !second_score.is_finite() {
        0.85
    } else {
        let raw = ((best_score / second_score) - 1.0).clamp(0.0, 2.0) / 2.0;
        (0.15 + 0.85 * raw).clamp(0.0, 1.0)
    };

    (best_bpm, confidence, octave_corrected)
}

fn tempo_prior(bpm: f32) -> f32 {
    // Mild preference for dance-music midrange so half-time peaks lose to doubles.
    let d = (bpm - 128.0) / 45.0;
    (-0.5 * d * d).exp()
}

/// Onset-phase beat-grid offset in `[0, beat)` seconds for a known BPM.
///
/// Sweeps candidate phases and scores onset-envelope energy on the grid.
fn estimate_beat_offset(samples: &[f32], sample_rate: u32, bpm: f32) -> f64 {
    let beat = 60.0 / bpm.max(1.0) as f64;
    if !beat.is_finite() || beat <= 0.0 {
        return 0.0;
    }
    let hop = (sample_rate as usize / 100).max(1); // ~10 ms
    let mut env = Vec::new();
    let mut prev = 0.0f32;
    for chunk in samples.chunks(hop) {
        let e: f32 = chunk.iter().map(|s| s.abs()).sum::<f32>() / chunk.len() as f32;
        env.push((e - prev).max(0.0));
        prev = e;
    }
    if env.len() < 4 {
        return 0.0;
    }
    let hop_secs = hop as f64 / sample_rate as f64;
    let beat_hops = (beat / hop_secs).round().max(1.0) as usize;
    if beat_hops == 0 || beat_hops >= env.len() {
        return 0.0;
    }

    let steps = beat_hops.clamp(8, 64);
    let mut best_off = 0.0f64;
    let mut best_score = f32::MIN;
    for s in 0..steps {
        let off_hops = (s * beat_hops) / steps;
        let mut score = 0.0f32;
        let mut i = off_hops;
        while i < env.len() {
            score += env[i];
            i += beat_hops;
        }
        if score > best_score {
            best_score = score;
            best_off = off_hops as f64 * hop_secs;
        }
    }
    best_off.rem_euclid(beat)
}

/// Returns `(pitch_class, minor, confidence)`.
fn estimate_key(samples: &[f32], sample_rate: u32) -> (u8, bool, f32) {
    let base = 110.0f32; // A2
    let mut chroma = [0.0f64; 12];
    // Up to 8 s (was 4 s) for stabler chroma on short loops.
    let n = samples.len().min(sample_rate as usize * 8);
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

    // Softmax-normalize for confidence.
    let max_raw = chroma.iter().cloned().fold(0.0f64, f64::max).max(1e-12);
    let mut soft = [0.0f64; 12];
    let mut denom = 0.0f64;
    for i in 0..12 {
        soft[i] = ((chroma[i] / max_raw) * 8.0).exp();
        denom += soft[i];
    }
    for s in soft.iter_mut() {
        *s /= denom;
    }

    let mut best = 0usize;
    let mut second = 0usize;
    for i in 1..12 {
        if soft[i] > soft[best] {
            second = best;
            best = i;
        } else if soft[i] > soft[second] {
            second = i;
        }
    }
    let confidence = if soft[best] <= 1e-9 {
        0.0
    } else {
        ((soft[best] - soft[second]) / soft[best]).clamp(0.0, 1.0) as f32
    };

    let major_third = chroma[(best + 4) % 12];
    let minor_third = chroma[(best + 3) % 12];
    let minor = minor_third > major_third;
    (best as u8, minor, confidence)
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
    use std::path::PathBuf;

    fn gold_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/gold")
    }

    fn click_train(sr: u32, bpm: f32, seconds: f64) -> Vec<f32> {
        click_train_offset(sr, bpm, seconds, 0.0)
    }

    fn click_train_offset(sr: u32, bpm: f32, seconds: f64, offset_secs: f64) -> Vec<f32> {
        let n = (sr as f64 * seconds) as usize;
        let beat = 60.0 / bpm as f64;
        let mut samples = vec![0.0f32; n];
        for i in 0..n {
            let t = i as f64 / sr as f64;
            let phase = (t - offset_secs).rem_euclid(beat) / beat;
            let click = if phase < 0.04 {
                (1.0 - phase / 0.04) as f32 * 0.9
            } else {
                0.0
            };
            samples[i] = click;
        }
        samples
    }

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
        assert!(!feats.quality.sparse_audio);
        assert!(feats.quality.key_confidence >= 0.0);
    }

    #[test]
    fn bpm_140_not_halved_to_70() {
        let samples = click_train(44_100, 140.0, 8.0);
        let feats = analyze_mono(&samples, 44_100).unwrap();
        assert!(
            (feats.bpm - 140.0).abs() < 2.0,
            "expected ~140, got {} (octave_corrected={})",
            feats.bpm,
            feats.quality.bpm_octave_corrected
        );
    }

    #[test]
    fn beat_offset_recovers_delayed_click_train() {
        let sr = 44_100u32;
        let bpm = 128.0f32;
        let delay = 0.10f64; // 100 ms into the beat (~469 ms)
        let samples = click_train_offset(sr, bpm, 8.0, delay);
        // Algorithm under known BPM (BPM autocorr can be ±1–2 on synthetic clicks).
        let got = estimate_beat_offset(&samples, sr, bpm);
        let beat = 60.0 / bpm as f64;
        let circ = {
            let d = (got - delay).rem_euclid(beat);
            d.min(beat - d)
        };
        assert!(
            circ <= 0.015,
            "offset {got} vs expected {delay} (circ err {circ}s)"
        );
        let feats = analyze_mono(&samples, sr).unwrap();
        assert!(feats.beat_offset_secs.is_finite());
        assert!((feats.bpm - bpm).abs() < 3.0, "bpm={}", feats.bpm);
        // When estimated BPM is close, analyze path should also recover.
        if (feats.bpm - bpm).abs() <= 1.0 {
            let circ2 = {
                let d = (feats.beat_offset_secs - delay).rem_euclid(beat);
                d.min(beat - d)
            };
            assert!(
                circ2 < 0.015,
                "analyze offset {} vs {delay} (circ {circ2})",
                feats.beat_offset_secs
            );
        }
    }

    #[test]
    fn sparse_silence_flags_quality() {
        let samples = vec![0.0f32; 44_100];
        let feats = analyze_mono(&samples, 44_100).unwrap();
        assert!(feats.quality.sparse_audio);
        assert!(feats.quality.bpm_confidence < 0.1);
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

    #[test]
    fn gold_analyze_matches_expected() {
        let expected_path = gold_dir().join("expected_analyze.json");
        let raw = std::fs::read_to_string(&expected_path)
            .unwrap_or_else(|e| panic!("missing {}: {e}", expected_path.display()));
        let expected: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let tracks = expected["tracks"].as_array().expect("tracks array");
        for row in tracks {
            let file = row["file"].as_str().unwrap();
            let path = gold_dir().join(file);
            let feats = analyze_path(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let bpm_exp = row["bpm"].as_f64().unwrap() as f32;
            assert!(
                (feats.bpm - bpm_exp).abs() <= 1.0,
                "{file}: bpm {} vs expected {bpm_exp}",
                feats.bpm
            );
            let key_exp = row["key"].as_str().unwrap();
            assert_eq!(feats.key.as_str(), key_exp, "{file}: key");
            assert!(
                feats.sections.intro_end < feats.sections.drop_start
                    && feats.sections.drop_start < feats.sections.outro_start,
                "{file}: section order {:?}",
                feats.sections
            );
            assert!(!feats.quality.sparse_audio, "{file}: sparse");
            assert!(
                feats.beat_offset_secs.is_finite() && feats.beat_offset_secs >= 0.0,
                "{file}: beat_offset={}",
                feats.beat_offset_secs
            );
        }
    }

}
