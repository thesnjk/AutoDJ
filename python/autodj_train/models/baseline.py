"""Tiny baseline transition classifier (no torch required)."""

from __future__ import annotations

from collections import Counter, defaultdict

from autodj_train.datasets.synthetic import TransitionExample, rule_label
from autodj_train.transitions import TransitionClass


class RuleBaseline:
    """Wraps the synthetic rule teacher — strong baseline for CI."""

    def predict_one(self, ex: TransitionExample) -> str:
        return rule_label(
            ex.bpm_a, ex.bpm_b, ex.camelot_dist, ex.energy_delta_db, ex.genre
        ).value

    def accuracy(self, examples: list[TransitionExample]) -> float:
        if not examples:
            return 0.0
        ok = sum(1 for ex in examples if self.predict_one(ex) == ex.label)
        return ok / len(examples)


class FrequencyPrior:
    """Predict most common label per genre from training counts."""

    def __init__(self) -> None:
        self._prior: dict[str, str] = {}

    def fit(self, examples: list[TransitionExample]) -> None:
        by_genre: dict[str, Counter[str]] = defaultdict(Counter)
        for ex in examples:
            by_genre[ex.genre][ex.label] += 1
        self._prior = {
            g: counts.most_common(1)[0][0] for g, counts in by_genre.items()
        }

    def predict_one(self, ex: TransitionExample) -> str:
        return self._prior.get(ex.genre, TransitionClass.LINEAR_CROSSFADE.value)
