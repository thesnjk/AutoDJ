"""Generate synthetic transition training pairs from rule heuristics."""

from __future__ import annotations

import json
import random
from dataclasses import asdict, dataclass
from pathlib import Path

from autodj_train.transitions import TransitionClass


@dataclass
class TransitionExample:
    """One labeled transition pair (metadata only — no audio bytes)."""

    bpm_a: float
    bpm_b: float
    camelot_dist: int
    energy_delta_db: float
    genre: str
    label: str


def rule_label(
    bpm_a: float,
    bpm_b: float,
    camelot_dist: int,
    energy_delta_db: float,
    genre: str,
) -> TransitionClass:
    """Deterministic rule teacher used to bootstrap the classifier."""
    bpm_delta = abs(bpm_a - bpm_b)
    if genre in {"drum_and_bass", "dnb"} or bpm_delta > 12:
        return TransitionClass.HARD_CUT
    if genre in {"hip_hop", "rnb"} and energy_delta_db < -1.0:
        return TransitionClass.ECHO_OUT
    if genre == "trance" or camelot_dist == 0:
        return TransitionClass.FILTER_SWEEP
    if camelot_dist <= 1 and bpm_delta < 4:
        return TransitionClass.EQ_SWAP
    return TransitionClass.LINEAR_CROSSFADE


def synthesize(n: int = 200, seed: int = 7) -> list[TransitionExample]:
    rng = random.Random(seed)
    genres = ["house", "drum_and_bass", "hip_hop", "trance", "uk_garage", "open_format"]
    out: list[TransitionExample] = []
    for _ in range(n):
        genre = rng.choice(genres)
        bpm_a = rng.uniform(80, 178)
        bpm_b = bpm_a + rng.uniform(-15, 15)
        camelot_dist = rng.randint(0, 6)
        energy_delta_db = rng.uniform(-6, 6)
        label = rule_label(bpm_a, bpm_b, camelot_dist, energy_delta_db, genre)
        out.append(
            TransitionExample(
                bpm_a=round(bpm_a, 2),
                bpm_b=round(bpm_b, 2),
                camelot_dist=camelot_dist,
                energy_delta_db=round(energy_delta_db, 2),
                genre=genre,
                label=label.value,
            )
        )
    return out


def write_jsonl(path: Path, examples: list[TransitionExample]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        for ex in examples:
            f.write(json.dumps(asdict(ex)) + "\n")
