"""Eval harness tests for transition baselines and tracklist aggregate."""

from __future__ import annotations

import json
from pathlib import Path

from autodj_train.datasets.synthetic import synthesize
from autodj_train.datasets.tracklist_stats import JsonFixtureSource, aggregate
from autodj_train.models.baseline import FrequencyPrior, RuleBaseline


def test_rule_baseline_perfect_on_synthetic():
    examples = synthesize(n=100, seed=1)
    model = RuleBaseline()
    assert model.accuracy(examples) == 1.0


def test_frequency_prior_trains():
    examples = synthesize(n=150, seed=2)
    model = FrequencyPrior()
    model.fit(examples)
    acc = sum(1 for ex in examples if model.predict_one(ex) == ex.label) / len(examples)
    assert acc >= 0.2


def test_tracklist_fixture_aggregate(tmp_path: Path):
    fixture = tmp_path / "obs.json"
    fixture.write_text(
        json.dumps(
            [
                {
                    "source_url": "https://example.com/set/1",
                    "retrieved_at": "2026-11-09",
                    "genre": "house",
                    "bpm_values": [124, 125, 126],
                    "key_routes": ["8A->9A", "9A->9B"],
                    "transition_density": 0.4,
                }
            ]
        ),
        encoding="utf-8",
    )
    stats = aggregate(JsonFixtureSource(fixture).fetch_observations())
    assert "house" in stats.genre_bpm_hist
    assert stats.key_route_counts["8A->9A"] == 1
