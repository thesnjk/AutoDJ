"""Eval harness tests for transition baselines, classifier, and tracklist aggregate."""

from __future__ import annotations

import json
from pathlib import Path

from autodj_train.datasets.synthetic import read_jsonl, synthesize, write_jsonl
from autodj_train.datasets.tracklist_stats import (
    CsvChartSource,
    JsonFixtureSource,
    aggregate,
    density_to_crossfade_secs,
    merge_observations,
    policy_hints_from_stats,
)
from autodj_train.models.baseline import FrequencyPrior, RuleBaseline
from autodj_train.models.features import stretch_clamp_excess
from autodj_train.models.softmax import SoftmaxLogReg
from autodj_train.transitions import ALL_CLASSES

FIXTURES = Path(__file__).resolve().parents[1] / "datasets" / "fixtures"
GOLD = Path(__file__).resolve().parent / "gold" / "synthetic_holdout.jsonl"
ARTIFACT = (
    Path(__file__).resolve().parents[1] / "models" / "artifacts" / "transition_model.json"
)


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


def test_balanced_synth_covers_all_classes():
    examples = synthesize(n=200, seed=3, balance_labels=True)
    labels = {ex.label for ex in examples}
    assert labels == {c.value for c in ALL_CLASSES}


def test_softmax_recovers_rules_on_holdout():
    # 8-class Dissolve needs a bit more capacity than the 7-class recipe.
    train = synthesize(n=20000, seed=7, balance_labels=True)
    model = SoftmaxLogReg(lr=0.25, epochs=20000, seed=7)
    model.fit(train)
    assert GOLD.exists(), "committed gold holdout missing"
    gold = read_jsonl(GOLD)
    assert RuleBaseline().accuracy(gold) == 1.0
    assert model.accuracy(gold) >= 0.90


def test_committed_artifact_passes_holdout():
    assert ARTIFACT.exists()
    model = SoftmaxLogReg.load(ARTIFACT)
    assert model.version >= 3
    assert len(model.mean) == 15
    assert len(model.classes) == 8
    assert "tremolo" in model.classes
    assert "riser" in model.classes
    assert "dissolve" in model.classes
    gold = read_jsonl(GOLD)
    assert model.accuracy(gold) >= 0.90


def test_stretch_clamp_excess_matches_rust():
    assert stretch_clamp_excess(128.0, 140.0) < 1e-9
    excess = stretch_clamp_excess(128.0, 155.0)
    assert abs(excess - (155.0 / 128.0 - 1.15)) < 1e-9


def test_jsonl_roundtrip(tmp_path: Path):
    examples = synthesize(n=20, seed=9)
    path = tmp_path / "t.jsonl"
    write_jsonl(path, examples)
    loaded = read_jsonl(path)
    assert len(loaded) == 20
    assert loaded[0].genre == examples[0].genre


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
