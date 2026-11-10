"""Pure-numpy multi-class logistic regression for transition classes."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import numpy as np

from autodj_train.datasets.synthetic import TransitionExample
from autodj_train.models.features import (
    DEFAULT_FEATURE_VERSION,
    FEATURE_GENRES,
    apply_standardizer,
    fit_standardizer,
    matrixize,
)
from autodj_train.transitions import ALL_CLASSES, TransitionClass

# All transition classes including Riser (committed Softmax is 7-class).
CLASS_ORDER = [c.value for c in ALL_CLASSES]


def _softmax(logits: np.ndarray) -> np.ndarray:
    shifted = logits - logits.max(axis=1, keepdims=True)
    exp = np.exp(shifted)
    return exp / exp.sum(axis=1, keepdims=True)


class SoftmaxLogReg:
    """Multi-class softmax classifier (numpy gradient descent)."""

    def __init__(
        self,
        *,
        lr: float = 0.35,
        epochs: int = 400,
        l2: float = 1e-4,
        seed: int = 0,
        version: int = DEFAULT_FEATURE_VERSION,
    ) -> None:
        self.lr = lr
        self.epochs = epochs
        self.l2 = l2
        self.seed = seed
        self.version = version
        self.classes: list[str] = list(CLASS_ORDER)
        self.genres: list[str] = list(FEATURE_GENRES)
        self.mean: np.ndarray | None = None
        self.std: np.ndarray | None = None
        self.weights: np.ndarray | None = None  # (C, F)
        self.bias: np.ndarray | None = None  # (C,)

    def fit(self, examples: list[TransitionExample]) -> SoftmaxLogReg:
        class_to_i = {c: i for i, c in enumerate(self.classes)}
        examples = [ex for ex in examples if ex.label in class_to_i]
        if not examples:
            raise ValueError("no training examples with Softmax-supported labels")
        x = matrixize(examples, version=self.version)
        y_labels = [ex.label for ex in examples]
        y = np.array([class_to_i[lab] for lab in y_labels], dtype=np.int64)

        self.mean, self.std = fit_standardizer(x)
        x = apply_standardizer(x, self.mean, self.std)

        rng = np.random.default_rng(self.seed)
        n, f = x.shape
        c = len(self.classes)
        self.weights = rng.normal(0.0, 0.01, size=(c, f))
        self.bias = np.zeros(c, dtype=np.float64)

        for _ in range(self.epochs):
            logits = x @ self.weights.T + self.bias
            probs = _softmax(logits)
            one_hot = np.zeros_like(probs)
            one_hot[np.arange(n), y] = 1.0
            grad = (probs - one_hot) / n
            dw = grad.T @ x + self.l2 * self.weights
            db = grad.sum(axis=0)
            self.weights -= self.lr * dw
            self.bias -= self.lr * db
        return self

    def predict_proba(self, examples: list[TransitionExample]) -> np.ndarray:
        assert self.weights is not None and self.mean is not None and self.std is not None
        x = apply_standardizer(
            matrixize(examples, version=self.version), self.mean, self.std
        )
        logits = x @ self.weights.T + self.bias
        return _softmax(logits)

    def predict(self, examples: list[TransitionExample]) -> list[str]:
        probs = self.predict_proba(examples)
        idx = probs.argmax(axis=1)
        return [self.classes[i] for i in idx]

    def predict_one(self, ex: TransitionExample) -> str:
        return self.predict([ex])[0]

    def accuracy(self, examples: list[TransitionExample]) -> float:
        # Score only Softmax-supported labels (e.g. exclude policy-only `riser`).
        supported = set(self.classes)
        examples = [ex for ex in examples if ex.label in supported]
        if not examples:
            return 0.0
        preds = self.predict(examples)
        ok = sum(1 for p, ex in zip(preds, examples, strict=True) if p == ex.label)
        return ok / len(examples)

    def to_dict(self) -> dict[str, Any]:
        assert self.weights is not None and self.mean is not None and self.std is not None
        assert self.bias is not None
        return {
            "version": int(self.version),
            "classes": self.classes,
            "genres": self.genres,
            "mean": self.mean.tolist(),
            "std": self.std.tolist(),
            "weights": self.weights.tolist(),
            "bias": self.bias.tolist(),
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> SoftmaxLogReg:
        version = int(data.get("version", 1))
        model = cls(version=version)
        model.classes = list(data["classes"])
        model.genres = list(data["genres"])
        model.mean = np.asarray(data["mean"], dtype=np.float64)
        model.std = np.asarray(data["std"], dtype=np.float64)
        model.weights = np.asarray(data["weights"], dtype=np.float64)
        model.bias = np.asarray(data["bias"], dtype=np.float64)
        return model

    def save(self, path: Path) -> None:
        import json

        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.to_dict(), indent=2) + "\n", encoding="utf-8")

    @classmethod
    def load(cls, path: Path) -> SoftmaxLogReg:
        import json

        data = json.loads(path.read_text(encoding="utf-8"))
        return cls.from_dict(data)


def label_enum(name: str) -> TransitionClass:
    return TransitionClass(name)
