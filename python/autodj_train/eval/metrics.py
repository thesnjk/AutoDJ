"""Eval metrics for the transition classifier (rubric train gate).

Maps to Docs/mix-quality-rubric.md automated row: ``autodj-train eval``
accuracy ≥ 0.90 on the frozen synthetic holdout (transition palette / genre policy).
"""

from __future__ import annotations

from collections import defaultdict

from autodj_train.datasets.synthetic import TransitionExample
from autodj_train.transitions import ALL_CLASSES


def accuracy(y_true: list[str], y_pred: list[str]) -> float:
    if not y_true:
        return 0.0
    return sum(1 for a, b in zip(y_true, y_pred, strict=True) if a == b) / len(y_true)


def per_class_recall(y_true: list[str], y_pred: list[str]) -> dict[str, float]:
    total: dict[str, int] = defaultdict(int)
    hits: dict[str, int] = defaultdict(int)
    for t, p in zip(y_true, y_pred, strict=True):
        total[t] += 1
        if t == p:
            hits[t] += 1
    out: dict[str, float] = {}
    for c in ALL_CLASSES:
        n = total[c.value]
        out[c.value] = (hits[c.value] / n) if n else 0.0
    return out


def evaluate_predictions(
    examples: list[TransitionExample], preds: list[str]
) -> dict[str, object]:
    labels = [ex.label for ex in examples]
    return {
        "n": len(examples),
        "accuracy": accuracy(labels, preds),
        "per_class_recall": per_class_recall(labels, preds),
    }
