//! Offline mix-quality eval helpers (spectral continuity).
//!
//! Pure Rust — Mac-native. Allocates freely; not for real-time audio threads.

use autodj_io::AudioBuffer;

/// Default max adjacent RMS jump for house-like continuous mixes.
pub const DEFAULT_MAX_FLUX_RATIO: f32 = 4.0;

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
}
