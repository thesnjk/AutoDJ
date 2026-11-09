"""Feature vectorization for transition examples."""

from __future__ import annotations

import numpy as np

from autodj_train.datasets.synthetic import GENRES, TransitionExample

# Fixed order — must match Rust TransitionModel.genres.
FEATURE_GENRES: list[str] = list(GENRES)
N_NUMERIC = 5  # bpm_a, bpm_b, |delta|, camelot_dist, energy_delta_db
FEATURE_DIM = N_NUMERIC + len(FEATURE_GENRES)


def vectorize(ex: TransitionExample) -> np.ndarray:
    """Return shape ``(FEATURE_DIM,)`` float64 feature vector."""
    vec = np.zeros(FEATURE_DIM, dtype=np.float64)
    vec[0] = ex.bpm_a
    vec[1] = ex.bpm_b
    vec[2] = abs(ex.bpm_a - ex.bpm_b)
    vec[3] = float(ex.camelot_dist)
    vec[4] = ex.energy_delta_db
    if ex.genre in FEATURE_GENRES:
        vec[N_NUMERIC + FEATURE_GENRES.index(ex.genre)] = 1.0
    return vec


def matrixize(examples: list[TransitionExample]) -> np.ndarray:
    if not examples:
        return np.zeros((0, FEATURE_DIM), dtype=np.float64)
    return np.stack([vectorize(ex) for ex in examples], axis=0)


def fit_standardizer(x: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """Return (mean, std) with std floored to avoid div-by-zero."""
    mean = x.mean(axis=0)
    std = x.std(axis=0)
    std = np.where(std < 1e-6, 1.0, std)
    return mean, std


def apply_standardizer(x: np.ndarray, mean: np.ndarray, std: np.ndarray) -> np.ndarray:
    return (x - mean) / std
