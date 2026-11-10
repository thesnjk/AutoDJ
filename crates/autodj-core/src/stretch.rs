//! Pitch-preserving time stretch (WSOLA) for offline tempo blend.
//!
//! Pure Rust — no platform audio backends. Safe on macOS Apple Silicon.
//! Allocates freely; do not call from a future real-time audio thread without
//! pre-warming buffers.

/// Minimum |ΔBPM| before tempo blend engages.
pub const TEMPO_BLEND_EPSILON_BPM: f32 = 0.5;

/// Soft clamp for stretch factors (~±15% tempo).
pub const STRETCH_MIN: f32 = 0.85;
/// Soft clamp upper bound (~±15% tempo).
pub const STRETCH_MAX: f32 = 1.15;

/// Stretch factor to apply to material at `to_bpm` so it matches `from_bpm`.
///
/// Values `> 1` lengthen (slow down) the material; `< 1` shorten (speed up).
pub fn tempo_ratio(from_bpm: f32, to_bpm: f32) -> f32 {
    let from = from_bpm.max(1.0);
    let to = to_bpm.max(1.0);
    // Slow faster tracks: 140→128 needs factor 140/128 > 1.
    to / from
}

fn clamp_stretch(factor: f32) -> f32 {
    factor.clamp(STRETCH_MIN, STRETCH_MAX)
}

/// How far `tempo_ratio(from, to)` sits outside [`STRETCH_MIN`, `STRETCH_MAX`] (0 = no clamp).
///
/// Used by the solver to cost warble-prone edges and by MixReport gates.
pub fn stretch_clamp_excess(from_bpm: f32, to_bpm: f32) -> f32 {
    let r = tempo_ratio(from_bpm, to_bpm);
    if r < STRETCH_MIN {
        STRETCH_MIN - r
    } else if r > STRETCH_MAX {
        r - STRETCH_MAX
    } else {
        0.0
    }
}

/// Whether the worst edge stretch clamp excess is zero (no silent WSOLA clip).
pub fn tempo_stretch_gate_pass(max_excess: f32) -> bool {
    max_excess.is_finite() && max_excess <= 0.0
}

/// Whether adjacent BPMs warrant tempo blend.
pub fn should_tempo_blend(bpm_a: f32, bpm_b: f32) -> bool {
    (bpm_a - bpm_b).abs() >= TEMPO_BLEND_EPSILON_BPM
}

/// Hann window of length `n`.
fn hann(n: usize) -> Vec<f32> {
    if n == 0 {
        return vec![];
    }
    (0..n)
        .map(|i| {
            let x = std::f32::consts::PI * i as f32 / n.saturating_sub(1).max(1) as f32;
            0.5 - 0.5 * (2.0 * x).cos()
        })
        .collect()
}

/// Cross-correlation lag search for best overlap alignment.
fn best_offset(ref_frame: &[f32], cand: &[f32], max_lag: usize) -> usize {
    let n = ref_frame.len().min(cand.len());
    if n < 8 || max_lag == 0 {
        return 0;
    }
    let mut best_lag = 0usize;
    let mut best_corr = f32::MIN;
    let search = max_lag.min(cand.len().saturating_sub(n));
    for lag in 0..=search {
        let mut corr = 0.0f32;
        for i in 0..n {
            corr += ref_frame[i] * cand[lag + i];
        }
        if corr > best_corr {
            best_corr = corr;
            best_lag = lag;
        }
    }
    best_lag
}

/// Pitch-preserving WSOLA time stretch.
///
/// `stretch_factor > 1` produces a longer (slower) output. `frame_ms` is the
/// analysis frame size in milliseconds; overlap is the fractional overlap (0.5 = 50%).
pub fn wsola_stretch(
    input: &[f32],
    sample_rate: u32,
    stretch_factor: f32,
    frame_ms: f32,
    overlap: f32,
) -> Vec<f32> {
    if input.is_empty() {
        return vec![];
    }
    let factor = stretch_factor.max(0.25);
    if (factor - 1.0).abs() < 1e-3 {
        return input.to_vec();
    }

    let sr = sample_rate.max(1) as f32;
    let frame_len = ((frame_ms.max(5.0) * 0.001 * sr) as usize).max(32);
    let overlap = overlap.clamp(0.25, 0.75);
    let analysis_hop = ((frame_len as f32) * (1.0 - overlap)).max(1.0) as usize;
    let synthesis_hop = ((analysis_hop as f32) * factor).max(1.0) as usize;
    let max_lag = ((0.005 * sr) as usize).max(1).min(analysis_hop);

    let window = hann(frame_len);
    let out_len = ((input.len() as f32) * factor).round().max(1.0) as usize;
    let mut output = vec![0.0f32; out_len + frame_len];
    let mut norm = vec![0.0f32; out_len + frame_len];

    let mut in_pos = 0usize;
    let mut out_pos = 0usize;

    while out_pos + frame_len < output.len() && in_pos + frame_len < input.len() {
        let mut lag = 0usize;
        if out_pos > 0 && in_pos + frame_len + max_lag < input.len() {
            // Reference: previous output overlap region.
            let ref_start = out_pos;
            let ref_len = analysis_hop.min(frame_len);
            if ref_start + ref_len <= output.len() {
                let reference: Vec<f32> = output[ref_start..ref_start + ref_len].to_vec();
                let cand_end = (in_pos + frame_len + max_lag).min(input.len());
                if cand_end > in_pos + ref_len {
                    lag = best_offset(&reference, &input[in_pos..cand_end], max_lag);
                }
            }
        }

        let src = in_pos + lag;
        if src + frame_len > input.len() {
            break;
        }
        for i in 0..frame_len {
            let w = window[i];
            output[out_pos + i] += input[src + i] * w;
            norm[out_pos + i] += w;
        }

        in_pos += analysis_hop;
        out_pos += synthesis_hop;
    }

    // Normalize overlapped regions.
    for i in 0..output.len() {
        if norm[i] > 1e-6 {
            output[i] /= norm[i];
        }
    }

    output.truncate(out_len);
    output
}

/// Fit `samples` to exactly `target_len` via linear interpolation.
pub fn fit_length(samples: &[f32], target_len: usize) -> Vec<f32> {
    if target_len == 0 {
        return vec![];
    }
    if samples.is_empty() {
        return vec![0.0; target_len];
    }
    if samples.len() == target_len {
        return samples.to_vec();
    }
    let mut out = vec![0.0f32; target_len];
    let last = (samples.len() - 1) as f32;
    for (i, sample) in out.iter_mut().enumerate() {
        let src = if target_len == 1 {
            0.0
        } else {
            i as f32 * last / (target_len - 1) as f32
        };
        let i0 = src.floor() as usize;
        let i1 = (i0 + 1).min(samples.len() - 1);
        let frac = src - i0 as f32;
        *sample = samples[i0] * (1.0 - frac) + samples[i1] * frac;
    }
    out
}

/// Piecewise WSOLA ramp over 8 segments from `start_factor` → `end_factor`, fitted to `samples.len()`.
fn tempo_blend_ramp(
    samples: &[f32],
    sample_rate: u32,
    start_factor: f32,
    end_factor: f32,
) -> Vec<f32> {
    let target_len = samples.len();
    const SEGMENTS: usize = 8;
    let seg_in = (samples.len() / SEGMENTS).max(1);
    let mut pieces: Vec<f32> = Vec::with_capacity(samples.len() + seg_in);
    for s in 0..SEGMENTS {
        let start = s * seg_in;
        if start >= samples.len() {
            break;
        }
        let end = if s + 1 == SEGMENTS {
            samples.len()
        } else {
            ((s + 1) * seg_in).min(samples.len())
        };
        let t = if SEGMENTS <= 1 {
            0.0
        } else {
            s as f32 / (SEGMENTS - 1) as f32
        };
        let factor = start_factor + (end_factor - start_factor) * t;
        let chunk = &samples[start..end];
        let stretched = wsola_stretch(chunk, sample_rate, factor, 30.0, 0.5);
        pieces.extend_from_slice(&stretched);
    }
    fit_length(&pieces, target_len)
}

/// Tempo-blend an incoming crossfade head from `bpm_a` (match outgoing) toward `bpm_b` (native).
///
/// Returns a buffer the same length as `b_head`. When `|bpm_a - bpm_b| < epsilon`, returns a copy.
pub fn tempo_blend_head(b_head: &[f32], sample_rate: u32, bpm_a: f32, bpm_b: f32) -> Vec<f32> {
    if b_head.is_empty() || !should_tempo_blend(bpm_a, bpm_b) {
        return b_head.to_vec();
    }
    let start_factor = clamp_stretch(tempo_ratio(bpm_a, bpm_b));
    let end_factor = 1.0f32;
    tempo_blend_ramp(b_head, sample_rate, start_factor, end_factor)
}

/// Tempo-blend an outgoing crossfade tail from native `bpm_a` toward `bpm_b` (incoming).
///
/// Mirrors djay Automix Sync + Tempo Blend on the outgoing deck. Returns a buffer the same
/// length as `a_tail`. When `|bpm_a - bpm_b| < epsilon`, returns a copy.
pub fn tempo_blend_tail(a_tail: &[f32], sample_rate: u32, bpm_a: f32, bpm_b: f32) -> Vec<f32> {
    if a_tail.is_empty() || !should_tempo_blend(bpm_a, bpm_b) {
        return a_tail.to_vec();
    }
    let start_factor = 1.0f32;
    let end_factor = clamp_stretch(tempo_ratio(bpm_b, bpm_a));
    tempo_blend_ramp(a_tail, sample_rate, start_factor, end_factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(sr: u32, secs: f32, freq: f32) -> Vec<f32> {
        let n = (sr as f32 * secs) as usize;
        (0..n)
            .map(|i| {
                let t = i as f32 / sr as f32;
                (2.0 * std::f32::consts::PI * freq * t).sin() * 0.3
            })
            .collect()
    }

    #[test]
    fn tempo_ratio_slows_faster_track() {
        let r = tempo_ratio(128.0, 140.0);
        assert!(r > 1.0, "140→128 should slow (stretch>1), got {r}");
        assert!((r - 140.0 / 128.0).abs() < 1e-5);
    }

    #[test]
    fn wsola_stretch_lengthens_by_factor() {
        let sr = 44_100u32;
        let input = tone(sr, 1.0, 220.0);
        let out = wsola_stretch(&input, sr, 1.1, 30.0, 0.5);
        let expected = (input.len() as f32 * 1.1) as isize;
        let delta = (out.len() as isize - expected).unsigned_abs();
        assert!(
            delta < (input.len() as f32 * 0.05) as usize,
            "len={} expected≈{}",
            out.len(),
            expected
        );
        assert!(out.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn identity_stretch_preserves_length() {
        let sr = 44_100u32;
        let input = tone(sr, 0.5, 440.0);
        let out = wsola_stretch(&input, sr, 1.0, 30.0, 0.5);
        assert_eq!(out.len(), input.len());
    }

    #[test]
    fn tempo_blend_128_to_140_keeps_length() {
        let sr = 44_100u32;
        let head = tone(sr, 1.0, 220.0);
        let out = tempo_blend_head(&head, sr, 128.0, 140.0);
        assert_eq!(out.len(), head.len());
        assert!(out.iter().all(|s| s.is_finite()));
        assert!(should_tempo_blend(128.0, 140.0));
        assert!(!should_tempo_blend(128.0, 128.2));
    }

    #[test]
    fn tempo_blend_tail_128_to_140_keeps_length() {
        let sr = 44_100u32;
        let tail = tone(sr, 1.0, 220.0);
        let out = tempo_blend_tail(&tail, sr, 128.0, 140.0);
        assert_eq!(out.len(), tail.len());
        assert!(out.iter().all(|s| s.is_finite()));
        let noop = tempo_blend_tail(&tail, sr, 128.0, 128.2);
        assert_eq!(noop, tail);
    }

    #[test]
    fn stretch_clamp_excess_in_range_is_zero() {
        assert_eq!(stretch_clamp_excess(128.0, 128.0), 0.0);
        assert!(
            stretch_clamp_excess(128.0, 140.0) < 1e-6,
            "128→140 (~1.094) must be inside ±15%"
        );
        assert!(tempo_stretch_gate_pass(0.0));
    }

    #[test]
    fn stretch_clamp_excess_outside_range() {
        let excess = stretch_clamp_excess(128.0, 150.0);
        assert!(
            excess > 0.01,
            "128→150 (~1.172) should clamp, got {excess}"
        );
        assert!(!tempo_stretch_gate_pass(excess));
        let slow = stretch_clamp_excess(140.0, 100.0);
        assert!(slow > 0.01, "140→100 should clamp low, got {slow}");
    }
}
