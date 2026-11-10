//! Offline 3-band EQ / transition DSP for AutoDJ.
//!
//! Pure Rust — no platform audio backends. Safe on macOS Apple Silicon and Linux CI.
//! Filter state may allocate; do not call from a future real-time audio thread without
//! pre-warming buffers.

/// Per-band linear gains in `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandGains {
    /// Low band (< ~250 Hz).
    pub low: f32,
    /// Mid band.
    pub mid: f32,
    /// High band (> ~2.5 kHz).
    pub high: f32,
}

/// Outgoing / incoming band gains for an EQ bass-swap at normalized time `t` in `[0, 1]`.
///
/// Bass swaps linearly (1→0 / 0→1). Mid/high use equal-power or linear amplitude crossfade.
pub fn eq_swap_gains(t: f32, equal_power: bool) -> (BandGains, BandGains) {
    let t = t.clamp(0.0, 1.0);
    let (mid_a, mid_b) = if equal_power {
        ((1.0 - t).sqrt(), t.sqrt())
    } else {
        (1.0 - t, t)
    };
    (
        BandGains {
            low: 1.0 - t,
            mid: mid_a,
            high: mid_a,
        },
        BandGains {
            low: t,
            mid: mid_b,
            high: mid_b,
        },
    )
}

/// Simple one-pole low-pass / high-pass.
#[derive(Debug, Clone)]
struct OnePole {
    a: f32,
    z: f32,
}

impl OnePole {
    fn lowpass(cutoff_hz: f32, sample_rate: u32) -> Self {
        let sr = sample_rate.max(1) as f32;
        let x = (-2.0 * std::f32::consts::PI * cutoff_hz.max(1.0) / sr).exp();
        Self { a: 1.0 - x, z: 0.0 }
    }

    fn process_lp(&mut self, x: f32) -> f32 {
        self.z += self.a * (x - self.z);
        self.z
    }

    fn process_hp(&mut self, x: f32) -> f32 {
        let lp = self.process_lp(x);
        x - lp
    }

    fn set_cutoff(&mut self, cutoff_hz: f32, sample_rate: u32) {
        let sr = sample_rate.max(1) as f32;
        let x = (-2.0 * std::f32::consts::PI * cutoff_hz.max(1.0) / sr).exp();
        self.a = 1.0 - x;
    }
}

/// Cascaded one-pole 3-band splitter (~250 Hz / ~2.5 kHz).
#[derive(Debug, Clone)]
pub struct ThreeBandSplitter {
    sample_rate: u32,
    low_lp: OnePole,
    high_lp: OnePole,
}

impl ThreeBandSplitter {
    /// Create a splitter for `sample_rate` Hz.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            low_lp: OnePole::lowpass(250.0, sample_rate),
            high_lp: OnePole::lowpass(2500.0, sample_rate),
        }
    }

    /// Split one sample into `(low, mid, high)`.
    pub fn process(&mut self, x: f32) -> (f32, f32, f32) {
        let low = self.low_lp.process_lp(x);
        let high_lp = self.high_lp.process_lp(x);
        let mid = high_lp - low;
        let high = x - high_lp;
        (low, mid, high)
    }

    /// Sample rate this splitter was built for.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

/// Apply EQ bass-swap crossfade over mono tails; returns mono wet samples.
pub fn apply_eq_swap_crossfade(
    a_tail: &[f32],
    b_head: &[f32],
    sample_rate: u32,
    equal_power: bool,
) -> Vec<f32> {
    let n = a_tail.len().min(b_head.len());
    let mut out = Vec::with_capacity(n);
    let mut split_a = ThreeBandSplitter::new(sample_rate);
    let mut split_b = ThreeBandSplitter::new(sample_rate);
    for i in 0..n {
        let t = if n <= 1 {
            1.0
        } else {
            i as f32 / (n - 1) as f32
        };
        let (ga, gb) = eq_swap_gains(t, equal_power);
        let (la, ma, ha) = split_a.process(a_tail[i]);
        let (lb, mb, hb) = split_b.process(b_head[i]);
        let s = la * ga.low
            + ma * ga.mid
            + ha * ga.high
            + lb * gb.low
            + mb * gb.mid
            + hb * gb.high;
        out.push(s);
    }
    out
}

/// Filter-sweep crossfade: outgoing high-pass rises; incoming low-pass opens.
pub fn apply_filter_sweep_crossfade(
    a_tail: &[f32],
    b_head: &[f32],
    sample_rate: u32,
    equal_power: bool,
) -> Vec<f32> {
    let n = a_tail.len().min(b_head.len());
    let mut out = Vec::with_capacity(n);
    let mut hp = OnePole::lowpass(80.0, sample_rate);
    let mut lp = OnePole::lowpass(200.0, sample_rate);
    for i in 0..n {
        let t = if n <= 1 {
            1.0
        } else {
            i as f32 / (n - 1) as f32
        };
        // Outgoing HP cutoff 80 → 8 kHz; incoming LP 200 → 16 kHz.
        let hp_cut = 80.0 + t * 7920.0;
        let lp_cut = 200.0 + t * 15800.0;
        hp.set_cutoff(hp_cut, sample_rate);
        lp.set_cutoff(lp_cut, sample_rate);
        let a = hp.process_hp(a_tail[i]);
        let b = lp.process_lp(b_head[i]);
        let (ga, gb) = if equal_power {
            ((1.0 - t).sqrt(), t.sqrt())
        } else {
            (1.0 - t, t)
        };
        out.push(a * ga + b * gb);
    }
    out
}

/// Echo-out crossfade: decaying feedback delay on outgoing; incoming equal-power in.
pub fn apply_echo_out_crossfade(
    a_tail: &[f32],
    b_head: &[f32],
    sample_rate: u32,
    equal_power: bool,
    delay_secs: f32,
) -> Vec<f32> {
    let n = a_tail.len().min(b_head.len());
    let mut out = Vec::with_capacity(n);
    let delay_samples = ((delay_secs.max(0.05) * sample_rate as f32) as usize)
        .max(1)
        .min(sample_rate as usize);
    let mut delay = vec![0.0f32; delay_samples];
    let mut w = 0usize;
    let feedback = 0.45f32;
    for i in 0..n {
        let t = if n <= 1 {
            1.0
        } else {
            i as f32 / (n - 1) as f32
        };
        let delayed = delay[w];
        let dry = a_tail[i];
        let wet = dry + delayed * feedback * (1.0 - t);
        delay[w] = wet;
        w = (w + 1) % delay_samples;

        let (ga, gb) = if equal_power {
            ((1.0 - t).sqrt(), t.sqrt())
        } else {
            (1.0 - t, t)
        };
        out.push(wet * ga + b_head[i] * gb);
    }
    out
}

/// Tremolo-out crossfade: beat-synced LFO amplitude on outgoing; incoming equal-power in.
///
/// Outgoing sample gain is `(0.5 + 0.5 * sin(2π · rate_hz · t_secs)) * ga` so depth is full
/// while the equal-power fade still hands the mix to B.
pub fn apply_tremolo_out_crossfade(
    a_tail: &[f32],
    b_head: &[f32],
    sample_rate: u32,
    equal_power: bool,
    rate_hz: f32,
) -> Vec<f32> {
    let n = a_tail.len().min(b_head.len());
    let mut out = Vec::with_capacity(n);
    let sr = sample_rate.max(1) as f32;
    let rate = rate_hz.max(0.1);
    let two_pi = std::f32::consts::TAU;
    for i in 0..n {
        let t = if n <= 1 {
            1.0
        } else {
            i as f32 / (n - 1) as f32
        };
        let t_secs = i as f32 / sr;
        let lfo = 0.5 + 0.5 * (two_pi * rate * t_secs).sin();
        let (ga, gb) = if equal_power {
            ((1.0 - t).sqrt(), t.sqrt())
        } else {
            (1.0 - t, t)
        };
        out.push(a_tail[i] * lfo * ga + b_head[i] * gb);
    }
    out
}

/// Linear band-kill envelope: gain `1→0` by normalized time `kill_by`.
fn dissolve_band_gain(t: f32, kill_by: f32) -> f32 {
    let kill_by = kill_by.max(1e-6);
    if t >= kill_by {
        0.0
    } else {
        (1.0 - t / kill_by).clamp(0.0, 1.0)
    }
}

/// Dissolve-out crossfade: staggered three-band evaporate on outgoing; incoming equal-power in.
///
/// Bass dies by `t≈0.45`, mid by `t≈0.75`, high by `t=1`, then the equal-power fade hands
/// the mix to B — distinct from [`apply_eq_swap_crossfade`] (bass handoff) and
/// [`apply_filter_sweep_crossfade`] (HP/LP).
pub fn apply_dissolve_out_crossfade(
    a_tail: &[f32],
    b_head: &[f32],
    sample_rate: u32,
    equal_power: bool,
) -> Vec<f32> {
    let n = a_tail.len().min(b_head.len());
    let mut out = Vec::with_capacity(n);
    let mut split_a = ThreeBandSplitter::new(sample_rate);
    for i in 0..n {
        let t = if n <= 1 {
            1.0
        } else {
            i as f32 / (n - 1) as f32
        };
        let (la, ma, ha) = split_a.process(a_tail[i]);
        let evaporated = la * dissolve_band_gain(t, 0.45)
            + ma * dissolve_band_gain(t, 0.75)
            + ha * dissolve_band_gain(t, 1.0);
        let (ga, gb) = if equal_power {
            ((1.0 - t).sqrt(), t.sqrt())
        } else {
            (1.0 - t, t)
        };
        out.push(evaporated * ga + b_head[i] * gb);
    }
    out
}

/// Riser-out crossfade: rising deterministic noise + outgoing high-pass; incoming equal-power in.
///
/// Noise amplitude follows `t²` (peak gain ~0.2) so energy builds before the swap without
/// clipping gold loops. Outgoing HP cutoff rises 80 Hz → 10 kHz. Noise is an LCG keyed by
/// sample index for golden-stable renders.
pub fn apply_riser_out_crossfade(
    a_tail: &[f32],
    b_head: &[f32],
    sample_rate: u32,
    equal_power: bool,
) -> Vec<f32> {
    let n = a_tail.len().min(b_head.len());
    let mut out = Vec::with_capacity(n);
    let mut hp = OnePole::lowpass(80.0, sample_rate);
    const NOISE_PEAK: f32 = 0.2;
    for i in 0..n {
        let t = if n <= 1 {
            1.0
        } else {
            i as f32 / (n - 1) as f32
        };
        let hp_cut = 80.0 + t * 9920.0;
        hp.set_cutoff(hp_cut, sample_rate);
        let a = hp.process_hp(a_tail[i]);
        // Deterministic LCG: x_{n+1} = (a x_n + c) mod 2^32 → [-1, 1].
        let mut state = (i as u32).wrapping_mul(1664525).wrapping_add(1013904223);
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = ((state >> 8) as f32 / (1u32 << 24) as f32) * 2.0 - 1.0;
        let noise_gain = NOISE_PEAK * t * t;
        let (ga, gb) = if equal_power {
            ((1.0 - t).sqrt(), t.sqrt())
        } else {
            (1.0 - t, t)
        };
        out.push((a + noise * noise_gain) * ga + b_head[i] * gb);
    }
    out
}

/// Band-limited RMS of `samples` via a temporary splitter (test / diagnostics).
pub fn band_rms(samples: &[f32], sample_rate: u32) -> (f32, f32, f32) {
    let mut split = ThreeBandSplitter::new(sample_rate);
    let mut sl = 0.0f64;
    let mut sm = 0.0f64;
    let mut sh = 0.0f64;
    for &x in samples {
        let (l, m, h) = split.process(x);
        sl += (l as f64).powi(2);
        sm += (m as f64).powi(2);
        sh += (h as f64).powi(2);
    }
    let n = samples.len().max(1) as f64;
    (
        (sl / n).sqrt() as f32,
        (sm / n).sqrt() as f32,
        (sh / n).sqrt() as f32,
    )
}

/// Max concurrent low-band ownership during a healthy EQ swap (~0.25 at mid-crossfade).
///
/// Tuned above the theoretical mid-swap product so house golden stays green, while dual
/// full-gain bass ownership (~1.0) hard-fails.
pub const DEFAULT_BASS_OVERLAP_LIMIT: f32 = 0.40;

/// Hop size for bass-overlap scoring (milliseconds).
const BASS_OVERLAP_HOP_MS: f32 = 40.0;

/// Max concurrent low-band ownership product during an EQ bass-swap (lower = cleaner handoff).
///
/// Per hop: `(ga.low * low_a) * (gb.low * low_b) / ref²` where `ref` is the louder deck's
/// unweighted low RMS. Empty/short tails return `0.0` (pass).
pub fn max_eq_swap_bass_overlap(
    a_tail: &[f32],
    b_head: &[f32],
    sample_rate: u32,
    equal_power: bool,
) -> f32 {
    score_bass_overlap(a_tail, b_head, sample_rate, |t| {
        let (ga, gb) = eq_swap_gains(t, equal_power);
        (ga.low, gb.low)
    })
}

/// Whether bass overlap is under the configured limit.
pub fn bass_conflict_pass(overlap: f32, max_allowed: f32) -> bool {
    overlap.is_finite() && overlap <= max_allowed
}

fn score_bass_overlap(
    a_tail: &[f32],
    b_head: &[f32],
    sample_rate: u32,
    low_gains: impl Fn(f32) -> (f32, f32),
) -> f32 {
    let n = a_tail.len().min(b_head.len());
    if n < 2 {
        return 0.0;
    }
    let hop = ((BASS_OVERLAP_HOP_MS * 0.001) * sample_rate.max(1) as f32)
        .round()
        .max(1.0) as usize;
    let mut worst = 0.0f32;
    let mut i = 0usize;
    while i < n {
        let end = (i + hop).min(n);
        let a_hop = &a_tail[i..end];
        let b_hop = &b_head[i..end];
        let (low_a, _, _) = band_rms(a_hop, sample_rate);
        let (low_b, _, _) = band_rms(b_hop, sample_rate);
        let mid = (i + end) as f32 * 0.5 / (n - 1) as f32;
        let (ga_low, gb_low) = low_gains(mid.clamp(0.0, 1.0));
        let ref_rms = low_a.max(low_b).max(1e-6);
        let contrib_a = ga_low.max(0.0) * low_a;
        let contrib_b = gb_low.max(0.0) * low_b;
        let score = (contrib_a * contrib_b) / (ref_rms * ref_rms);
        if score > worst {
            worst = score;
        }
        if end == n {
            break;
        }
        i = end;
    }
    worst
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(sr: u32, secs: f32, freq: f32, amp: f32) -> Vec<f32> {
        let n = (sr as f32 * secs) as usize;
        (0..n)
            .map(|i| {
                let t = i as f32 / sr as f32;
                (2.0 * std::f32::consts::PI * freq * t).sin() * amp
            })
            .collect()
    }

    #[test]
    fn eq_swap_moves_bass_from_a_to_b() {
        let sr = 44_100u32;
        // Outgoing = bass (60 Hz); incoming = also bass but we'll compare early vs late wet.
        let a = tone(sr, 2.0, 60.0, 0.5);
        let b = tone(sr, 2.0, 60.0, 0.5);
        // Phase-offset B so they aren't identical when mixed.
        let b: Vec<f32> = b
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let t = i as f32 / sr as f32;
                (2.0 * std::f32::consts::PI * 60.0 * t + 1.0).sin() * 0.5
            })
            .collect();
        let wet = apply_eq_swap_crossfade(&a, &b, sr, true);
        let early = &wet[..wet.len() / 8];
        let late = &wet[wet.len() * 7 / 8..];
        let (e_low, _, _) = band_rms(early, sr);
        let (l_low, _, _) = band_rms(late, sr);
        // Both ends have bass energy; early should track A (gain~1), late B (gain~1).
        assert!(e_low > 0.05, "early low rms={e_low}");
        assert!(l_low > 0.05, "late low rms={l_low}");
        // Midpoint gains: at t=0 outgoing low=1, at t=1 outgoing low=0.
        let (g0, _) = eq_swap_gains(0.0, true);
        let (g1, h1) = eq_swap_gains(1.0, true);
        assert!((g0.low - 1.0).abs() < 1e-5);
        assert!(g1.low < 1e-5);
        assert!((h1.low - 1.0).abs() < 1e-5);
    }

    #[test]
    fn filter_sweep_and_echo_are_finite() {
        let sr = 44_100u32;
        let a = tone(sr, 1.0, 220.0, 0.3);
        let b = tone(sr, 1.0, 440.0, 0.3);
        let sweep = apply_filter_sweep_crossfade(&a, &b, sr, true);
        let echo = apply_echo_out_crossfade(&a, &b, sr, true, 0.375);
        assert_eq!(sweep.len(), a.len().min(b.len()));
        assert!(sweep.iter().all(|s| s.is_finite()));
        assert!(echo.iter().all(|s| s.is_finite()));
        let peak = echo.iter().copied().map(f32::abs).fold(0.0, f32::max);
        assert!(peak < 5.0); // before limiter; should not explode
    }

    #[test]
    fn tremolo_out_is_finite() {
        let sr = 44_100u32;
        let a = tone(sr, 1.0, 220.0, 0.3);
        let b = tone(sr, 1.0, 440.0, 0.3);
        let trem = apply_tremolo_out_crossfade(&a, &b, sr, true, 2.0);
        assert_eq!(trem.len(), a.len().min(b.len()));
        assert!(trem.iter().all(|s| s.is_finite()));
        let peak = trem.iter().copied().map(f32::abs).fold(0.0, f32::max);
        assert!(peak < 5.0);
    }

    #[test]
    fn riser_out_is_finite_and_builds_energy() {
        let sr = 44_100u32;
        // Quiet outgoing so the noise bed dominates early-half energy growth.
        let a = tone(sr, 1.0, 220.0, 0.02);
        let b = tone(sr, 1.0, 440.0, 0.3);
        let riser = apply_riser_out_crossfade(&a, &b, sr, true);
        assert_eq!(riser.len(), a.len().min(b.len()));
        assert!(riser.iter().all(|s| s.is_finite()));
        let peak = riser.iter().copied().map(f32::abs).fold(0.0, f32::max);
        assert!(peak < 5.0, "riser peak={peak}");
        let mid = riser.len() / 2;
        let early = &riser[..mid / 2];
        let late_first_half = &riser[mid / 2..mid];
        let rms = |xs: &[f32]| {
            let s: f64 = xs.iter().map(|&x| (x as f64) * (x as f64)).sum();
            (s / xs.len().max(1) as f64).sqrt()
        };
        let early_rms = rms(early);
        let late_rms = rms(late_first_half);
        assert!(
            late_rms > early_rms * 1.5,
            "expected riser energy build early_rms={early_rms} late_rms={late_rms}"
        );
    }

    #[test]
    fn dissolve_out_is_finite_and_evaporates() {
        let sr = 44_100u32;
        // Broadband-ish outgoing (low + high); silent B so residual is A only.
        let a: Vec<f32> = (0..(sr as usize))
            .map(|i| {
                let t = i as f32 / sr as f32;
                let low = (2.0 * std::f32::consts::PI * 80.0 * t).sin() * 0.4;
                let high = (2.0 * std::f32::consts::PI * 4000.0 * t).sin() * 0.4;
                low + high
            })
            .collect();
        let b = vec![0.0f32; a.len()];
        let dissolve = apply_dissolve_out_crossfade(&a, &b, sr, true);
        assert_eq!(dissolve.len(), a.len());
        assert!(dissolve.iter().all(|s| s.is_finite()));
        let peak = dissolve.iter().copied().map(f32::abs).fold(0.0, f32::max);
        assert!(peak < 5.0, "dissolve peak={peak}");

        let n = dissolve.len();
        let early = &dissolve[..n / 8];
        let late = &dissolve[n * 7 / 8..];
        let rms = |xs: &[f32]| {
            let s: f64 = xs.iter().map(|&x| (x as f64) * (x as f64)).sum();
            (s / xs.len().max(1) as f64).sqrt()
        };
        let early_rms = rms(early);
        let late_rms = rms(late);
        assert!(
            late_rms < early_rms * 0.35,
            "expected dissolve evaporate early_rms={early_rms} late_rms={late_rms}"
        );

        // High-band residual (post one-pole HP ~2.5 kHz) should also fall.
        let mut early_hp = OnePole::lowpass(2500.0, sr);
        let mut late_hp = OnePole::lowpass(2500.0, sr);
        let early_hf: f64 = early
            .iter()
            .map(|&x| {
                let h = early_hp.process_hp(x);
                (h as f64) * (h as f64)
            })
            .sum::<f64>()
            / early.len().max(1) as f64;
        let late_hf: f64 = late
            .iter()
            .map(|&x| {
                let h = late_hp.process_hp(x);
                (h as f64) * (h as f64)
            })
            .sum::<f64>()
            / late.len().max(1) as f64;
        assert!(
            late_hf.sqrt() < early_hf.sqrt() * 0.5,
            "expected HF kill early_hf={} late_hf={}",
            early_hf.sqrt(),
            late_hf.sqrt()
        );
    }

    #[test]
    fn clean_eq_swap_bass_overlap_passes() {
        let sr = 44_100u32;
        let a = tone(sr, 2.0, 60.0, 0.5);
        let b: Vec<f32> = (0..a.len())
            .map(|i| {
                let t = i as f32 / sr as f32;
                (2.0 * std::f32::consts::PI * 60.0 * t + 1.0).sin() * 0.5
            })
            .collect();
        let overlap = max_eq_swap_bass_overlap(&a, &b, sr, true);
        assert!(
            bass_conflict_pass(overlap, DEFAULT_BASS_OVERLAP_LIMIT),
            "clean swap overlap={overlap}"
        );
        assert!(overlap > 0.05, "expected mid-swap ownership, got {overlap}");
    }

    #[test]
    fn dual_full_gain_bass_fails_conflict_gate() {
        let sr = 44_100u32;
        let a = tone(sr, 1.0, 60.0, 0.6);
        let b = tone(sr, 1.0, 55.0, 0.6);
        let overlap = score_bass_overlap(&a, &b, sr, |_| (1.0, 1.0));
        assert!(
            !bass_conflict_pass(overlap, DEFAULT_BASS_OVERLAP_LIMIT),
            "dual full-gain overlap should fail, got {overlap}"
        );
        assert!(overlap > 0.8, "expected near-1.0 dual ownership, got {overlap}");
    }

    #[test]
    fn empty_tails_bass_overlap_is_zero() {
        assert_eq!(max_eq_swap_bass_overlap(&[], &[], 44_100, true), 0.0);
        assert!(bass_conflict_pass(0.0, DEFAULT_BASS_OVERLAP_LIMIT));
    }
}
