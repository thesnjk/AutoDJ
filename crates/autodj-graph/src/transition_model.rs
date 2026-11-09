//! Softmax transition classifier loaded from Python `autodj-train` JSON artifacts.

use autodj_core::TransitionClass;
use serde::{Deserialize, Serialize};

/// Serialized SoftmaxLogReg weights (matches `SoftmaxLogReg.to_dict`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionModel {
    /// Schema version.
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

impl TransitionModel {
    /// Parse model JSON.
    pub fn from_json(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
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

/// Build the same feature vector as Python `vectorize` (length = 5 + n_genres).
pub fn feature_vector(
    model: &TransitionModel,
    bpm_a: f32,
    bpm_b: f32,
    camelot_dist: u8,
    energy_delta_db: f32,
    genre: &str,
) -> Vec<f64> {
    let n = 5 + model.genres.len();
    let mut v = vec![0.0_f64; n];
    v[0] = f64::from(bpm_a);
    v[1] = f64::from(bpm_b);
    v[2] = f64::from((bpm_a - bpm_b).abs());
    v[3] = f64::from(camelot_dist);
    v[4] = f64::from(energy_delta_db);
    if let Some(gi) = model.genres.iter().position(|g| g == genre) {
        v[5 + gi] = 1.0;
    }
    v
}

/// Softmax argmax prediction matching Python `SoftmaxLogReg.predict`.
pub fn predict_transition_class(
    model: &TransitionModel,
    bpm_a: f32,
    bpm_b: f32,
    camelot_dist: u8,
    energy_delta_db: f32,
    genre: &str,
) -> TransitionClass {
    let raw = feature_vector(model, bpm_a, bpm_b, camelot_dist, energy_delta_db, genre);
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
