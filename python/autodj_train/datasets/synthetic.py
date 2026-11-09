"""Generate synthetic transition training pairs from rule heuristics."""

from __future__ import annotations

import json
import random
from collections import defaultdict
from dataclasses import asdict, dataclass
from pathlib import Path

from autodj_train.transitions import ALL_CLASSES, TransitionClass

# Genre-aware BPM bands matching Docs/genre-policy.md.
GENRE_BPM_BANDS: dict[str, tuple[float, float]] = {
    "house": (120.0, 130.0),
    "drum_and_bass": (170.0, 178.0),
    "hip_hop": (80.0, 100.0),
    "trance": (132.0, 140.0),
    "uk_garage": (128.0, 138.0),
    "open_format": (90.0, 140.0),
}

GENRES = list(GENRE_BPM_BANDS.keys())


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


def _sample_one(rng: random.Random, genre: str | None = None) -> TransitionExample:
    genre = genre or rng.choice(GENRES)
    lo, hi = GENRE_BPM_BANDS[genre]
    bpm_a = rng.uniform(lo, hi)
    # Allow jumps outside the band so hard_cut / blend diversity appears.
    bpm_b = bpm_a + rng.uniform(-18.0, 18.0)
    bpm_b = max(70.0, min(180.0, bpm_b))
    camelot_dist = rng.randint(0, 6)
    energy_delta_db = rng.uniform(-6.0, 6.0)
    label = rule_label(bpm_a, bpm_b, camelot_dist, energy_delta_db, genre)
    return TransitionExample(
        bpm_a=round(bpm_a, 2),
        bpm_b=round(bpm_b, 2),
        camelot_dist=camelot_dist,
        energy_delta_db=round(energy_delta_db, 2),
        genre=genre,
        label=label.value,
    )


def synthesize(
    n: int = 200,
    seed: int = 7,
    *,
    balance_labels: bool = False,
) -> list[TransitionExample]:
    """Synthesize ``n`` labeled pairs.

    When ``balance_labels`` is True, keep sampling until each transition class
    has at least ``n // len(ALL_CLASSES)`` examples (total may slightly exceed ``n``).
    """
    rng = random.Random(seed)
    if not balance_labels:
        return [_sample_one(rng) for _ in range(n)]

    per_class = max(1, n // len(ALL_CLASSES))
    buckets: dict[str, list[TransitionExample]] = defaultdict(list)
    # Hard cap to avoid infinite loops if a class is unreachable.
    attempts = 0
    max_attempts = n * 40
    while attempts < max_attempts:
        attempts += 1
        ex = _sample_one(rng)
        if len(buckets[ex.label]) < per_class:
            buckets[ex.label].append(ex)
        if all(len(buckets[c.value]) >= per_class for c in ALL_CLASSES):
            break
    out: list[TransitionExample] = []
    for c in ALL_CLASSES:
        out.extend(buckets[c.value])
    rng.shuffle(out)
    # Top up to at least n if some classes were scarce.
    while len(out) < n:
        out.append(_sample_one(rng))
    return out[: max(n, per_class * len(ALL_CLASSES))]


def write_jsonl(path: Path, examples: list[TransitionExample]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        for ex in examples:
            f.write(json.dumps(asdict(ex)) + "\n")


def read_jsonl(path: Path) -> list[TransitionExample]:
    examples: list[TransitionExample] = []
    with path.open(encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            examples.append(
                TransitionExample(
                    bpm_a=float(row["bpm_a"]),
                    bpm_b=float(row["bpm_b"]),
                    camelot_dist=int(row["camelot_dist"]),
                    energy_delta_db=float(row["energy_delta_db"]),
                    genre=str(row["genre"]),
                    label=str(row["label"]),
                )
            )
    return examples
