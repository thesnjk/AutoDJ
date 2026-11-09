//! Offline mix renderer: beat-aligned crossfade + true-peak-ish limiter.

use autodj_io::AudioBuffer;

use crate::analysis::TransitionClass;
use crate::eq::{
    apply_echo_out_crossfade, apply_eq_swap_crossfade, apply_filter_sweep_crossfade,
};
use crate::stretch::{should_tempo_blend, tempo_blend_head};

/// One segment in a rendered mix.
#[derive(Debug, Clone)]
pub struct MixSegment {
    /// Source PCM (mono or stereo; renderer downmixes/upmixes to stereo).
    pub buffer: AudioBuffer,
    /// BPM used for beat alignment.
    pub bpm: f32,
    /// Crossfade length into the next track (seconds). Last segment ignores.
    pub crossfade_secs: f32,
    /// Transition style into the next track.
    pub transition: TransitionClass,
}

/// Full mix plan.
#[derive(Debug, Clone)]
pub struct MixPlan {
    /// Ordered segments.
    pub segments: Vec<MixSegment>,
    /// Target sample rate.
    pub sample_rate: u32,
}

/// Crossfade shaping.
#[derive(Debug, Clone, Copy)]
pub struct CrossfadeParams {
    /// Equal-power crossfade when true; linear when false.
    pub equal_power: bool,
}

impl Default for CrossfadeParams {
    fn default() -> Self {
        Self { equal_power: true }
    }
}

/// Result of rendering a mix, including whether tempo blend ran.
#[derive(Debug, Clone)]
pub struct MixRenderResult {
    /// Interleaved stereo PCM.
    pub buffer: AudioBuffer,
    /// True if any crossfade applied WSOLA tempo blend (|ΔBPM| ≥ epsilon).
    pub tempo_blend_applied: bool,
}

/// Render a [`MixPlan`] to an interleaved stereo buffer, then limit peaks.
pub fn mix_buffers(plan: &MixPlan, params: CrossfadeParams) -> AudioBuffer {
    mix_buffers_with_meta(plan, params).buffer
}

/// Render a mix and report whether tempo blend was applied.
pub fn mix_buffers_with_meta(plan: &MixPlan, params: CrossfadeParams) -> MixRenderResult {
    let sr = plan.sample_rate;
    let mut out: Vec<f32> = Vec::new();
    let mut tempo_blend_applied = false;

    if plan.segments.is_empty() {
        return MixRenderResult {
            buffer: AudioBuffer {
                sample_rate: sr,
                channels: 2,
                samples: out,
            },
            tempo_blend_applied: false,
        };
    }

    let monos: Vec<Vec<f32>> = plan
        .segments
        .iter()
        .map(|seg| to_mono_resampled(&seg.buffer, sr))
        .collect();

    for (idx, seg) in plan.segments.iter().enumerate() {
        let mono = &monos[idx];
        let next = plan.segments.get(idx + 1);
        let xfade = if next.is_some() {
            let next_mono = &monos[idx + 1];
            clamp_xfade_samples(
                seg.crossfade_secs,
                sr,
                mono.len(),
                next_mono.len(),
            )
        } else {
            0
        };

        if idx == 0 {
            let body_end = mono.len().saturating_sub(xfade);
            append_mono_as_stereo(&mut out, &mono[..body_end]);
        }

        if let Some(next_seg) = next {
            let next_mono = &monos[idx + 1];
            let a_tail = &mono[mono.len() - xfade..];
            let b_raw = &next_mono[..xfade];
            if should_tempo_blend(seg.bpm, next_seg.bpm) {
                tempo_blend_applied = true;
            }
            let b_head = tempo_blend_head(b_raw, sr, seg.bpm, next_seg.bpm);
            let wet = crossfade_mono(
                a_tail,
                &b_head,
                sr,
                seg.transition,
                params.equal_power,
                seg.bpm,
            );
            append_mono_as_stereo(&mut out, &wet);
            let next_xfade = if idx + 2 < plan.segments.len() {
                clamp_xfade_samples(
                    next_seg.crossfade_secs,
                    sr,
                    next_mono.len(),
                    monos[idx + 2].len(),
                )
            } else {
                0
            };
            let body_end = next_mono.len().saturating_sub(next_xfade);
            if xfade <= body_end {
                append_mono_as_stereo(&mut out, &next_mono[xfade..body_end]);
            }
        }
    }

    limit_inplace(&mut out, 0.99);
    MixRenderResult {
        buffer: AudioBuffer {
            sample_rate: sr,
            channels: 2,
            samples: out,
        },
        tempo_blend_applied,
    }
}

/// Cap crossfade so short fixtures cannot request a fade longer than ~35% of either track.
fn clamp_xfade_samples(
    crossfade_secs: f32,
    sample_rate: u32,
    len_a: usize,
    len_b: usize,
) -> usize {
    let requested = (crossfade_secs.max(0.0) * sample_rate as f32) as usize;
    let max_allowed = ((len_a.min(len_b) as f32) * 0.35) as usize;
    requested
        .min(max_allowed)
        .min(len_a.saturating_sub(1))
        .min(len_b.saturating_sub(1))
}

/// Render a mono crossfade region according to transition class.
fn crossfade_mono(
    a_tail: &[f32],
    b_head: &[f32],
    sample_rate: u32,
    class: TransitionClass,
    equal_power: bool,
    bpm: f32,
) -> Vec<f32> {
    match class {
        TransitionClass::EqSwap => {
            apply_eq_swap_crossfade(a_tail, b_head, sample_rate, equal_power)
        }
        TransitionClass::FilterSweep => {
            apply_filter_sweep_crossfade(a_tail, b_head, sample_rate, equal_power)
        }
        TransitionClass::EchoOut => {
            let delay_secs = (60.0 / bpm.max(1.0) * 1.5).clamp(0.12, 0.5);
            apply_echo_out_crossfade(a_tail, b_head, sample_rate, equal_power, delay_secs)
        }
        TransitionClass::HardCut | TransitionClass::LinearCrossfade => {
            let n = a_tail.len().min(b_head.len());
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let t = if n <= 1 {
                    1.0
                } else {
                    i as f32 / (n - 1) as f32
                };
                let (ga, gb) = gains_for(class, t, equal_power);
                out.push(a_tail[i] * ga + b_head[i] * gb);
            }
            out
        }
    }
}

fn gains_for(class: TransitionClass, t: f32, equal_power: bool) -> (f32, f32) {
    match class {
        TransitionClass::HardCut => {
            if t < 0.5 {
                (1.0, 0.0)
            } else {
                (0.0, 1.0)
            }
        }
        TransitionClass::LinearCrossfade
        | TransitionClass::EqSwap
        | TransitionClass::FilterSweep
        | TransitionClass::EchoOut => {
            if equal_power {
                ((1.0 - t).sqrt(), t.sqrt())
            } else {
                (1.0 - t, t)
            }
        }
    }
}

fn append_mono_as_stereo(out: &mut Vec<f32>, mono: &[f32]) {
    out.reserve(mono.len() * 2);
    for &s in mono {
        out.push(s);
        out.push(s);
    }
}

fn to_mono_resampled(buf: &AudioBuffer, target_sr: u32) -> Vec<f32> {
    let mono = buf.to_mono();
    if buf.sample_rate == target_sr || mono.is_empty() {
        return mono;
    }
    // Linear resample (MVP).
    let ratio = target_sr as f64 / buf.sample_rate as f64;
    let out_len = (mono.len() as f64 * ratio) as usize;
    let mut out = vec![0.0f32; out_len];
    for (i, sample) in out.iter_mut().enumerate() {
        let src = i as f64 / ratio;
        let i0 = src.floor() as usize;
        let i1 = (i0 + 1).min(mono.len() - 1);
        let frac = (src - i0 as f64) as f32;
        *sample = mono[i0] * (1.0 - frac) + mono[i1] * frac;
    }
    out
}

fn limit_inplace(samples: &mut [f32], ceiling: f32) {
    let mut peak = 0.0f32;
    for &s in samples.iter() {
        peak = peak.max(s.abs());
    }
    if peak <= ceiling || peak < 1e-9 {
        return;
    }
    let g = ceiling / peak;
    for s in samples.iter_mut() {
        *s *= g;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::TransitionClass;
    use autodj_io::AudioBuffer;

    fn tone(sr: u32, secs: f32, freq: f32) -> AudioBuffer {
        let n = (sr as f32 * secs) as usize;
        let mut samples = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / sr as f32;
            samples.push((2.0 * std::f32::consts::PI * freq * t).sin() * 0.3);
        }
        AudioBuffer {
            sample_rate: sr,
            channels: 1,
            samples,
        }
    }

    #[test]
    fn renders_two_segment_mix() {
        let sr = 44100;
        let plan = MixPlan {
            sample_rate: sr,
            segments: vec![
                MixSegment {
                    buffer: tone(sr, 2.0, 220.0),
                    bpm: 128.0,
                    crossfade_secs: 0.5,
                    transition: TransitionClass::LinearCrossfade,
                },
                MixSegment {
                    buffer: tone(sr, 2.0, 330.0),
                    bpm: 128.0,
                    crossfade_secs: 0.0,
                    transition: TransitionClass::HardCut,
                },
            ],
        };
        let out = mix_buffers(&plan, CrossfadeParams::default());
        assert_eq!(out.channels, 2);
        assert!(out.frames() > sr as usize);
        let peak = out.samples.iter().copied().map(f32::abs).fold(0.0, f32::max);
        assert!(peak <= 1.0);
    }

    #[test]
    fn eq_swap_mix_stays_finite() {
        let sr = 44100;
        let plan = MixPlan {
            sample_rate: sr,
            segments: vec![
                MixSegment {
                    buffer: tone(sr, 2.5, 60.0),
                    bpm: 128.0,
                    crossfade_secs: 0.8,
                    transition: TransitionClass::EqSwap,
                },
                MixSegment {
                    buffer: tone(sr, 2.5, 440.0),
                    bpm: 128.0,
                    crossfade_secs: 0.0,
                    transition: TransitionClass::HardCut,
                },
            ],
        };
        let out = mix_buffers(&plan, CrossfadeParams::default());
        assert!(out.samples.iter().all(|s| s.is_finite()));
        let peak = out.samples.iter().copied().map(f32::abs).fold(0.0, f32::max);
        assert!(peak <= 1.0);
    }

    #[test]
    fn tempo_blend_mismatch_bpm_stays_finite() {
        let sr = 44100;
        let plan = MixPlan {
            sample_rate: sr,
            segments: vec![
                MixSegment {
                    buffer: tone(sr, 2.0, 220.0),
                    bpm: 120.0,
                    crossfade_secs: 1.0,
                    transition: TransitionClass::LinearCrossfade,
                },
                MixSegment {
                    buffer: tone(sr, 2.0, 330.0),
                    bpm: 140.0,
                    crossfade_secs: 0.0,
                    transition: TransitionClass::HardCut,
                },
            ],
        };
        let result = mix_buffers_with_meta(&plan, CrossfadeParams::default());
        assert!(result.tempo_blend_applied);
        assert!(result.buffer.samples.iter().all(|s| s.is_finite()));
        let peak = result
            .buffer
            .samples
            .iter()
            .copied()
            .map(f32::abs)
            .fold(0.0, f32::max);
        assert!(peak <= 1.0);
    }
}
