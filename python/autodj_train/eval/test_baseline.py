"""Eval harness tests for transition baselines and tracklist aggregate."""

from __future__ import annotations

import json
from pathlib import Path

from autodj_train.datasets.synthetic import synthesize
from autodj_train.datasets.tracklist_stats import (
    CsvChartSource,
    JsonFixtureSource,
    aggregate,
    density_to_crossfade_secs,
    merge_observations,
    policy_hints_from_stats,
)
from autodj_train.models.baseline import FrequencyPrior, RuleBaseline

FIXTURES = Path(__file__).resolve().parents[1] / "datasets" / "fixtures"


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
    assert abs(stats.bpm_center_estimate["house"] - 125.0) < 1e-6
    assert abs(stats.mean_transition_density["house"] - 0.4) < 1e-6


def test_csv_chart_aggregate():
    csv_path = FIXTURES / "sample_charts.csv"
    stats = aggregate(CsvChartSource(csv_path).fetch_observations())
    assert "uk_garage" in stats.genre_bpm_hist
    assert stats.key_route_counts["5A->6A"] == 1
    assert abs(stats.bpm_center_estimate["uk_garage"] - 132.0) < 1e-6


def test_density_to_crossfade_mapping():
    assert density_to_crossfade_secs(1.2) == 0.5
    assert density_to_crossfade_secs(0.3) == 12.0
    mid = density_to_crossfade_secs(0.7)
    assert 0.5 < mid < 12.0


def test_policy_hints_from_merged_fixtures():
    obs = merge_observations(
        JsonFixtureSource(FIXTURES / "sample_tracklists.json"),
        JsonFixtureSource(FIXTURES / "curated_setcraft.json"),
        CsvChartSource(FIXTURES / "sample_charts.csv"),
    )
    stats = aggregate(obs)
    hints = policy_hints_from_stats(stats)
    assert "house" in hints
    assert "drum_and_bass" in hints
    assert hints["drum_and_bass"]["default_crossfade_secs"] == 0.5
    assert hints["house"]["default_crossfade_secs"] >= 10.0
    assert 120.0 <= hints["house"]["bpm_center"] <= 128.0
