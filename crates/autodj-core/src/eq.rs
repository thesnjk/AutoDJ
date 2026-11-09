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
}
