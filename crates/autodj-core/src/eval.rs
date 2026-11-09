//! Offline mix-quality eval helpers (spectral continuity + beat-phase).
//!
//! Pure Rust — Mac-native. Allocates freely; not for real-time audio threads.

use autodj_io::AudioBuffer;

use crate::cue::beat_secs;

/// Default max adjacent RMS jump for house-like continuous mixes.
pub const DEFAULT_MAX_FLUX_RATIO: f32 = 4.0;

/// Max adjacent hop-RMS ratio inside a crossfade neighborhood (looser than whole-mix).
pub const DEFAULT_JOIN_FLUX_RATIO: f32 = 6.0;

/// Half-width of the join scoring window in milliseconds.
const JOIN_WINDOW_HALF_MS: f32 = 100.0;

/// Hop size for join-local envelopes (milliseconds). Match whole-mix hop for stable RMS.
const JOIN_HOP_MS: f32 = 50.0;

/// Rubric limit: mean absolute downbeat join offset (milliseconds).
pub const DEFAULT_BEAT_PHASE_LIMIT_MS: f32 = 50.0;

/// True-peak ceiling matching the offline limiter (linear full-scale).
pub const DEFAULT_TRUE_PEAK_LIMIT: f64 = 0.99;

/// One segment's cue window + tempo grid for join-phase scoring.
#[derive(Debug, Clone, Copy)]
pub struct BeatPhaseCue {
    /// Mix-in start within the source track (seconds).
    pub play_start_secs: f64,
    /// Mix-out end within the source track (seconds).
    pub play_end_secs: f64,
    /// Outgoing crossfade into the next track (seconds).
    pub crossfade_secs: f32,
    /// Estimated BPM.
    pub bpm: f32,
    /// Beat-grid offset (`[0, beat)` seconds).
    pub beat_offset_secs: f64,
}

/// Minimum hops required before the spectral gate is enforced.
const MIN_HOPS: usize = 8;

/// Per-hop RMS envelope (hop in milliseconds).
pub fn spectral_flux_envelope(mono: &[f32], sample_rate: u32, hop_ms: f32) -> Vec<f32> {
    if mono.is_empty() {
        return vec![];
    }
    let hop = ((hop_ms.max(10.0) * 0.001) * sample_rate.max(1) as f32).round() as usize;
    let hop = hop.max(1);
    let mut env = Vec::new();
    for chunk in mono.chunks(hop) {
        let sum: f64 = chunk.iter().map(|s| (*s as f64).powi(2)).sum();
        env.push((sum / chunk.len() as f64).sqrt() as f32);
    }
    env
}

/// Continuity stress: max adjacent hop-RMS ratio `max(a,b)/min(a,b)`.
///
/// Returns `1.0` when the buffer is too short to evaluate (treated as pass).
pub fn adjacent_flux_ratio(buffer: &AudioBuffer) -> f32 {
    let mono = buffer.to_mono();
    let rms = spectral_flux_envelope(&mono, buffer.sample_rate, 50.0);
    if rms.len() < MIN_HOPS {
        return 1.0;
    }
    let mut worst = 1.0f32;
    for w in rms.windows(2) {
        let a = w[0].max(1e-9);
        let b = w[1].max(1e-9);
        let ratio = a.max(b) / a.min(b);
        if ratio > worst {
            worst = ratio;
        }
    }
    worst
}

/// Whether `ratio` is within the allowed continuity bound.
pub fn spectral_continuity_pass(ratio: f32, max_allowed: f32) -> bool {
    ratio.is_finite() && ratio <= max_allowed
}

/// Absolute beat-phase difference at one join, in milliseconds.
///
/// Compares outgoing phase at fade start vs incoming phase at play start.
/// Uses the **outgoing** beat period so multi-BPM edges stay comparable after WSOLA.
fn join_phase_offset_ms(out: &BeatPhaseCue, incoming: &BeatPhaseCue) -> f32 {
    let beat_out = beat_secs(out.bpm);
    if beat_out <= 0.0 || !beat_out.is_finite() {
        return 0.0;
    }
    let t_out = (out.play_end_secs - out.crossfade_secs as f64).max(out.play_start_secs);
    let t_in = incoming.play_start_secs;
    let p_out = (t_out - out.beat_offset_secs).rem_euclid(beat_out);
    let beat_in = beat_secs(incoming.bpm).max(1e-9);
    let p_in = (t_in - incoming.beat_offset_secs).rem_euclid(beat_in);
    // Normalize both to fraction-of-beat, then express |Δ| in outgoing ms.
    let frac_out = p_out / beat_out;
    let frac_in = p_in / beat_in;
    let mut d = frac_out - frac_in;
    if d > 0.5 {
        d -= 1.0;
    } else if d < -0.5 {
        d += 1.0;
    }
    (d.abs() * beat_out * 1000.0) as f32
}

/// Mean absolute join-phase offset across adjacent cue pairs (milliseconds).
///
/// Prefer same-tempo edges when any exist (multi-BPM joins are informational until
/// WSOLA phase-locks); otherwise average all edges. Empty / single-cue → `0.0`.
pub fn mean_beat_phase_offset_ms(cues: &[BeatPhaseCue]) -> f32 {
    if cues.len() < 2 {
        return 0.0;
    }
    let mut same = Vec::new();
    let mut all = Vec::new();
    for w in cues.windows(2) {
        let ms = join_phase_offset_ms(&w[0], &w[1]);
        all.push(ms);
        if (w[0].bpm - w[1].bpm).abs() < 0.5 {
            same.push(ms);
        }
    }
    let use_vals = if same.is_empty() { &all } else { &same };
    let sum: f32 = use_vals.iter().sum();
    sum / use_vals.len() as f32
}

/// Whether mean beat-phase offset is under the rubric limit.
pub fn beat_phase_gate_pass(mean_ms: f32, limit_ms: f32) -> bool {
    mean_ms.is_finite() && mean_ms < limit_ms
}

/// Whether measured true-peak is within the limiter ceiling.
pub fn peak_gate_pass(true_peak: f64, limit: f64) -> bool {
    true_peak.is_finite() && true_peak <= limit
}

/// Worst adjacent hop-RMS ratio inside ±[`JOIN_WINDOW_HALF_MS`] of each join.
///
/// `join_samples` are mono-frame indices into the rendered mix. Empty joins → `1.0` (pass).
pub fn max_join_flux_ratio(buffer: &AudioBuffer, join_samples: &[usize]) -> f32 {
    if join_samples.is_empty() {
        return 1.0;
    }
    let mono = buffer.to_mono();
    if mono.len() < 2 {
        return 1.0;
    }
    let sr = buffer.sample_rate.max(1) as f32;
    let half = ((JOIN_WINDOW_HALF_MS * 0.001) * sr).round() as usize;
    let min_window = (((JOIN_HOP_MS * 0.001) * sr).round() as usize).max(1) * 2;
    let mut worst = 1.0f32;
    for &join in join_samples {
        let start = join.saturating_sub(half).min(mono.len());
        let end = (join + half).min(mono.len());
        if end.saturating_sub(start) < min_window {
            continue;
        }
        let window = &mono[start..end];
        let env = spectral_flux_envelope(window, buffer.sample_rate, JOIN_HOP_MS);
        if env.len() < 2 {
            continue;
        }
        for w in env.windows(2) {
            let a = w[0].max(1e-9);
            let b = w[1].max(1e-9);
            let ratio = a.max(b) / a.min(b);
            if ratio > worst {
                worst = ratio;
            }
        }
    }
    worst
}

/// Whether join-local flux is within the allowed bound.
pub fn join_continuity_pass(ratio: f32, max_allowed: f32) -> bool {
    ratio.is_finite() && ratio <= max_allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone_buf(sr: u32, secs: f32, freq: f32) -> AudioBuffer {
        let n = (sr as f32 * secs) as usize;
        let samples: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / sr as f32;
                (2.0 * std::f32::consts::PI * freq * t).sin() * 0.2
            })
            .collect();
        AudioBuffer {
            sample_rate: sr,
            channels: 1,
            samples,
        }
    }

    #[test]
    fn smooth_tone_passes_continuity() {
        let buf = tone_buf(44_100, 2.0, 220.0);
        let ratio = adjacent_flux_ratio(&buf);
        assert!(
            spectral_continuity_pass(ratio, DEFAULT_MAX_FLUX_RATIO),
            "ratio={ratio}"
        );
    }

    #[test]
    fn abrupt_click_fails_continuity() {
        let sr = 44_100u32;
        let mut buf = tone_buf(sr, 2.0, 220.0);
        let mid = buf.samples.len() / 2;
        // Insert a near-full-scale burst spanning one hop.
        for i in mid..mid + (sr as usize / 20) {
            if i < buf.samples.len() {
                buf.samples[i] = if (i - mid) % 2 == 0 { 0.99 } else { -0.99 };
            }
        }
        let ratio = adjacent_flux_ratio(&buf);
        assert!(
            !spectral_continuity_pass(ratio, DEFAULT_MAX_FLUX_RATIO),
            "expected fail, ratio={ratio}"
        );
    }

    #[test]
    fn aligned_same_tempo_cues_pass_beat_phase() {
        let a = BeatPhaseCue {
            play_start_secs: 0.0,
            play_end_secs: 8.0,
            crossfade_secs: 2.0,
            bpm: 120.0,
            beat_offset_secs: 0.0,
        };
        let b = BeatPhaseCue {
            play_start_secs: 0.0,
            play_end_secs: 8.0,
            crossfade_secs: 0.0,
            bpm: 120.0,
            beat_offset_secs: 0.0,
        };
        let mean = mean_beat_phase_offset_ms(&[a, b]);
        assert!(mean < 1.0, "mean={mean}");
        assert!(beat_phase_gate_pass(mean, DEFAULT_BEAT_PHASE_LIMIT_MS));
    }

    #[test]
    fn misaligned_cues_fail_beat_phase() {
        let a = BeatPhaseCue {
            play_start_secs: 0.0,
            play_end_secs: 8.0,
            crossfade_secs: 2.0,
            bpm: 120.0,
            beat_offset_secs: 0.0,
        };
        let b = BeatPhaseCue {
            play_start_secs: 0.15,
            play_end_secs: 8.0,
            crossfade_secs: 0.0,
            bpm: 120.0,
            beat_offset_secs: 0.0,
        };
        let mean = mean_beat_phase_offset_ms(&[a, b]);
        assert!(mean > 50.0, "mean={mean}");
        assert!(!beat_phase_gate_pass(mean, DEFAULT_BEAT_PHASE_LIMIT_MS));
    }

    #[test]
    fn peak_gate_respects_ceiling() {
        assert!(peak_gate_pass(0.99, DEFAULT_TRUE_PEAK_LIMIT));
        assert!(!peak_gate_pass(1.01, DEFAULT_TRUE_PEAK_LIMIT));
    }

    #[test]
    fn smooth_join_passes_join_continuity() {
        let buf = tone_buf(44_100, 2.0, 220.0);
        let join = buf.samples.len() / 2;
        let ratio = max_join_flux_ratio(&buf, &[join]);
        assert!(
            join_continuity_pass(ratio, DEFAULT_JOIN_FLUX_RATIO),
            "ratio={ratio}"
        );
    }

    #[test]
    fn spiked_join_fails_join_continuity() {
        let sr = 44_100u32;
        let mut buf = tone_buf(sr, 2.0, 220.0);
        let join = buf.samples.len() / 2;
        // ~80 ms near-full-scale burst so a 50 ms hop neighborhood sees a sharp jump.
        for i in join..join + (sr as usize * 80 / 1000) {
            if i < buf.samples.len() {
                buf.samples[i] = if (i - join) % 2 == 0 { 0.99 } else { -0.99 };
            }
        }
        let ratio = max_join_flux_ratio(&buf, &[join]);
        assert!(
            !join_continuity_pass(ratio, DEFAULT_JOIN_FLUX_RATIO),
            "expected fail, ratio={ratio}"
        );
    }

    #[test]
    fn empty_joins_pass() {
        let buf = tone_buf(44_100, 1.0, 220.0);
        assert!((max_join_flux_ratio(&buf, &[]) - 1.0).abs() < 1e-6);
    }
}
