"""Feature vectorization for transition examples."""

from __future__ import annotations

import numpy as np

from autodj_train.datasets.synthetic import GENRES, TransitionExample

# Fixed order — must match Rust TransitionModel.genres.
FEATURE_GENRES: list[str] = list(GENRES)

# Soft clamp for stretch factors (~±15% tempo) — must match autodj_core::stretch.
STRETCH_MIN = 0.85
STRETCH_MAX = 1.15

N_NUMERIC_V1 = 5  # bpm_a, bpm_b, |delta|, camelot_dist, energy_delta_db
N_NUMERIC_V2 = 7  # + stretch_clamp_excess, spectral_centroid_delta_hz
N_NUMERIC_V3 = 9  # + bars_per_phrase_a, |bars_a − bars_b|
FEATURE_DIM_V1 = N_NUMERIC_V1 + len(FEATURE_GENRES)
FEATURE_DIM_V2 = N_NUMERIC_V2 + len(FEATURE_GENRES)
FEATURE_DIM_V3 = N_NUMERIC_V3 + len(FEATURE_GENRES)

# Default training / artifact schema.
N_NUMERIC = N_NUMERIC_V3
FEATURE_DIM = FEATURE_DIM_V3
DEFAULT_FEATURE_VERSION = 3

PHRASE_BARS = (4, 8, 16)


def normalize_phrase_bars(bars: float) -> float:
    """Clamp to {4,8,16}; invalid → 8 (matches Rust predict path)."""
    b = round(float(bars))
    if b in PHRASE_BARS:
        return float(b)
    return 8.0


def stretch_clamp_excess(bpm_a: float, bpm_b: float) -> float:
    """How far tempo_ratio sits outside ±15% clamp (0 = no clamp). Matches Rust."""
    from_bpm = max(float(bpm_a), 1.0)
    to_bpm = max(float(bpm_b), 1.0)
    r = to_bpm / from_bpm
    if r < STRETCH_MIN:
        return STRETCH_MIN - r
    if r > STRETCH_MAX:
        return r - STRETCH_MAX
    return 0.0


def feature_dim_for_version(version: int) -> int:
    if version <= 1:
        return FEATURE_DIM_V1
    if version == 2:
        return FEATURE_DIM_V2
    return FEATURE_DIM_V3


def n_numeric_for_version(version: int) -> int:
    if version <= 1:
        return N_NUMERIC_V1
    if version == 2:
        return N_NUMERIC_V2
    return N_NUMERIC_V3


def vectorize(ex: TransitionExample, version: int = DEFAULT_FEATURE_VERSION) -> np.ndarray:
    """Return float64 feature vector for the given schema version."""
    dim = feature_dim_for_version(version)
    n_num = n_numeric_for_version(version)
    vec = np.zeros(dim, dtype=np.float64)
    vec[0] = ex.bpm_a
    vec[1] = ex.bpm_b
    vec[2] = abs(ex.bpm_a - ex.bpm_b)
    vec[3] = float(ex.camelot_dist)
    vec[4] = ex.energy_delta_db
    if version >= 2:
        vec[5] = stretch_clamp_excess(ex.bpm_a, ex.bpm_b)
        ca = float(ex.spectral_centroid_hz_a)
        cb = float(ex.spectral_centroid_hz_b)
        vec[6] = cb - ca
    if version >= 3:
        # Linear Softmax needs an explicit mismatch dim (not raw bars_b alone).
        ba = normalize_phrase_bars(ex.bars_per_phrase_a)
        bb = normalize_phrase_bars(ex.bars_per_phrase_b)
        vec[7] = ba
        vec[8] = abs(ba - bb)
    if ex.genre in FEATURE_GENRES:
        vec[n_num + FEATURE_GENRES.index(ex.genre)] = 1.0
    return vec


def matrixize(
    examples: list[TransitionExample],
    version: int = DEFAULT_FEATURE_VERSION,
) -> np.ndarray:
    dim = feature_dim_for_version(version)
    if not examples:
        return np.zeros((0, dim), dtype=np.float64)
    return np.stack([vectorize(ex, version=version) for ex in examples], axis=0)


def fit_standardizer(x: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """Return (mean, std) with std floored to avoid div-by-zero."""
    mean = x.mean(axis=0)
    std = x.std(axis=0)
    std = np.where(std < 1e-6, 1.0, std)
    return mean, std


def apply_standardizer(x: np.ndarray, mean: np.ndarray, std: np.ndarray) -> np.ndarray:
    return (x - mean) / std
