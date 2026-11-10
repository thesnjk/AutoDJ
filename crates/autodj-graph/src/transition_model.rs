//! Softmax transition classifier loaded from Python `autodj-train` JSON artifacts.

use autodj_core::{stretch_clamp_excess, TrackFeatures, TransitionClass};
use serde::{Deserialize, Serialize};

/// Serialized SoftmaxLogReg weights (matches `SoftmaxLogReg.to_dict`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionModel {
    /// Schema version (`1` = 5 numeric; `2` = 7; `3` = 9 incl. phrase bars).
    #[serde(default = "default_version")]
    pub version: u32,
    /// Class labels in weight-row order (snake_case).
    pub classes: Vec<String>,
    /// Genre one-hot order matching Python `FEATURE_GENRES`.
    pub genres: Vec<String>,
    /// Feature means for standardization.
    pub mean: Vec<f64>,
    /// Feature stds for standardization.
    pub std: Vec<f64>,
    /// Weight matrix `[C][F]`.
    pub weights: Vec<Vec<f64>>,
    /// Bias vector `[C]`.
    pub bias: Vec<f64>,
}

fn default_version() -> u32 {
    1
}

fn normalize_phrase_bars(bars: u32) -> f32 {
    match bars {
        4 | 8 | 16 => bars as f32,
        _ => 8.0,
    }
}

impl TransitionModel {
    /// Parse model JSON.
    pub fn from_json(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }

    /// Numeric feature count before genre one-hot.
    pub fn n_numeric(&self) -> usize {
        if self.version >= 3 {
            9
        } else if self.version >= 2 {
            7
        } else {
            5
        }
    }
}

fn parse_class(name: &str) -> TransitionClass {
    match name {
        "linear_crossfade" => TransitionClass::LinearCrossfade,
        "eq_swap" => TransitionClass::EqSwap,
        "filter_sweep" => TransitionClass::FilterSweep,
        "echo_out" => TransitionClass::EchoOut,
        "hard_cut" => TransitionClass::HardCut,
        _ => TransitionClass::LinearCrossfade,
    }
}

/// Build the same feature vector as Python `vectorize` for this model version.
#[allow(clippy::too_many_arguments)]
pub fn feature_vector(
    model: &TransitionModel,
    bpm_a: f32,
    bpm_b: f32,
    camelot_dist: u8,
    energy_delta_db: f32,
    genre: &str,
    spectral_centroid_delta_hz: f32,
    bars_per_phrase_a: u32,
    bars_per_phrase_b: u32,
) -> Vec<f64> {
    let n_num = model.n_numeric();
    let n = n_num + model.genres.len();
    let mut v = vec![0.0_f64; n];
    v[0] = f64::from(bpm_a);
    v[1] = f64::from(bpm_b);
    v[2] = f64::from((bpm_a - bpm_b).abs());
    v[3] = f64::from(camelot_dist);
    v[4] = f64::from(energy_delta_db);
    if model.version >= 2 {
        v[5] = f64::from(stretch_clamp_excess(bpm_a, bpm_b));
        v[6] = f64::from(spectral_centroid_delta_hz);
    }
    if model.version >= 3 {
        // Match Python: bars_a + |bars_a − bars_b| (linear Softmax needs mismatch).
        let ba = normalize_phrase_bars(bars_per_phrase_a);
        let bb = normalize_phrase_bars(bars_per_phrase_b);
        v[7] = f64::from(ba);
        v[8] = f64::from((ba - bb).abs());
    }
    if let Some(gi) = model.genres.iter().position(|g| g == genre) {
        v[n_num + gi] = 1.0;
    }
    v
}

/// Softmax argmax from a pre-built raw feature vector.
fn predict_from_raw(model: &TransitionModel, raw: &[f64]) -> TransitionClass {
    if raw.len() != model.mean.len() || model.weights.is_empty() {
        return TransitionClass::LinearCrossfade;
    }
    let mut x = vec![0.0_f64; raw.len()];
    for i in 0..raw.len() {
        let std = if model.std[i].abs() < 1e-6 {
            1.0
        } else {
            model.std[i]
        };
        x[i] = (raw[i] - model.mean[i]) / std;
    }

    let c = model.weights.len();
    let mut logits = vec![0.0_f64; c];
    for (ci, row) in model.weights.iter().enumerate() {
        let mut s = model.bias.get(ci).copied().unwrap_or(0.0);
        for (fi, &w) in row.iter().enumerate() {
            if fi < x.len() {
                s += w * x[fi];
            }
        }
        logits[ci] = s;
    }

    let max_logit = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mut best_i = 0usize;
    let mut best_p = f64::NEG_INFINITY;
    let mut denom = 0.0_f64;
    let mut exps = vec![0.0_f64; c];
    for (i, &logit) in logits.iter().enumerate() {
        let e = (logit - max_logit).exp();
        exps[i] = e;
        denom += e;
    }
    for (i, e) in exps.iter().enumerate() {
        let p = e / denom;
        if p > best_p {
            best_p = p;
            best_i = i;
        }
    }
    let name = model
        .classes
        .get(best_i)
        .map(String::as_str)
        .unwrap_or("linear_crossfade");
    parse_class(name)
}

/// Softmax argmax prediction matching Python `SoftmaxLogReg.predict`.
///
/// Phrase bars and spectral delta are ignored on older schema versions. Prefer
/// [`predict_transition_class_edge`] when track features are available.
pub fn predict_transition_class(
    model: &TransitionModel,
    bpm_a: f32,
    bpm_b: f32,
    camelot_dist: u8,
    energy_delta_db: f32,
    genre: &str,
) -> TransitionClass {
    predict_transition_class_with_spectral(
        model,
        bpm_a,
        bpm_b,
        camelot_dist,
        energy_delta_db,
        genre,
        0.0,
        8,
        8,
    )
}

/// Softmax prediction with explicit spectral centroid delta and phrase bars.
#[allow(clippy::too_many_arguments)]
pub fn predict_transition_class_with_spectral(
    model: &TransitionModel,
    bpm_a: f32,
    bpm_b: f32,
    camelot_dist: u8,
    energy_delta_db: f32,
    genre: &str,
    spectral_centroid_delta_hz: f32,
    bars_per_phrase_a: u32,
    bars_per_phrase_b: u32,
) -> TransitionClass {
    let raw = feature_vector(
        model,
        bpm_a,
        bpm_b,
        camelot_dist,
        energy_delta_db,
        genre,
        spectral_centroid_delta_hz,
        bars_per_phrase_a,
        bars_per_phrase_b,
    );
    predict_from_raw(model, &raw)
}

/// Softmax prediction from a pair of analyzed tracks (v3 uses stretch + spectral + phrase).
pub fn predict_transition_class_edge(
    model: &TransitionModel,
    a: &TrackFeatures,
    b: &TrackFeatures,
    camelot_dist: u8,
    genre: &str,
) -> TransitionClass {
    predict_transition_class_with_spectral(
        model,
        a.bpm,
        b.bpm,
        camelot_dist,
        b.loudness_db - a.loudness_db,
        genre,
        b.spectral_centroid_hz - a.spectral_centroid_hz,
        a.bars_per_phrase,
        b.bars_per_phrase,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use autodj_core::stretch_clamp_excess;

    fn tiny_hard_cut_model() -> TransitionModel {
        // 5 numeric + 1 genre (drum_and_bass). Bias hard_cut high when genre one-hot is on.
        TransitionModel {
            version: 1,
            classes: vec![
                "linear_crossfade".into(),
                "eq_swap".into(),
                "filter_sweep".into(),
                "echo_out".into(),
                "hard_cut".into(),
            ],
            genres: vec!["drum_and_bass".into()],
            mean: vec![0.0; 6],
            std: vec![1.0; 6],
            weights: vec![
                vec![0.0; 6],
                vec![0.0; 6],
                vec![0.0; 6],
                vec![0.0; 6],
                vec![0.0, 0.0, 0.0, 0.0, 0.0, 5.0],
            ],
            bias: vec![0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    #[test]
    fn predicts_hard_cut_for_dnb_one_hot() {
        let m = tiny_hard_cut_model();
        let c = predict_transition_class(&m, 174.0, 175.0, 1, 0.0, "drum_and_bass");
        assert_eq!(c, TransitionClass::HardCut);
    }

    #[test]
    fn stretch_parity_with_core() {
        // 128→140 within ±15% → 0; 128→155 clamps.
        assert!(stretch_clamp_excess(128.0, 140.0) < 1e-6);
        let excess = stretch_clamp_excess(128.0, 155.0);
        assert!(excess > 0.0, "expected clamp excess, got {excess}");
        // Python: tempo_ratio = 155/128 ≈ 1.2109; excess ≈ 1.2109 - 1.15 ≈ 0.0609
        assert!((excess - (155.0 / 128.0 - 1.15)).abs() < 1e-5);
    }

    #[test]
    fn v1_fixture_loads_and_predicts() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../python/autodj_train/models/artifacts/transition_model_v1.json"
        );
        let Ok(raw) = std::fs::read_to_string(path) else {
            return;
        };
        let model = TransitionModel::from_json(&raw).expect("parse v1");
        assert_eq!(model.version, 1);
        assert_eq!(model.mean.len(), 5 + model.genres.len());
        let _ = predict_transition_class(&model, 124.0, 125.0, 1, 0.5, "house");
    }

    #[test]
    fn v2_fixture_loads_and_predicts() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../python/autodj_train/models/artifacts/transition_model_v2.json"
        );
        let Ok(raw) = std::fs::read_to_string(path) else {
            // Snapshot created later in the same PR.
            return;
        };
        let model = TransitionModel::from_json(&raw).expect("parse v2");
        assert_eq!(model.version, 2);
        assert_eq!(model.mean.len(), 7 + model.genres.len());
        let _ = predict_transition_class(&model, 124.0, 125.0, 1, 0.5, "house");
    }

    #[test]
    fn v3_feature_vector_includes_phrase_bars() {
        let m = TransitionModel {
            version: 3,
            classes: vec!["hard_cut".into()],
            genres: vec!["house".into()],
            mean: vec![0.0; 10],
            std: vec![1.0; 10],
            weights: vec![vec![0.0; 10]],
            bias: vec![0.0],
        };
        let v = feature_vector(&m, 124.0, 125.0, 1, 0.5, "house", 100.0, 4, 16);
        assert_eq!(v.len(), 10);
        assert!((v[7] - 4.0).abs() < 1e-9);
        assert!((v[8] - 12.0).abs() < 1e-9); // |4 − 16|
    }
}
