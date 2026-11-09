"""Tracklist / chart statistics ingest (aggregate only)."""

from __future__ import annotations

import csv
import json
import statistics
from collections.abc import Iterable
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Protocol


@dataclass
class SetObservation:
    """One attributable setcraft observation (no audio)."""

    source_url: str
    retrieved_at: str
    genre: str
    bpm_values: list[float] = field(default_factory=list)
    key_routes: list[str] = field(default_factory=list)
    transition_density: float = 0.0  # transitions per minute estimate


@dataclass
class AggregateStats:
    """Commit-safe aggregate dataset."""

    genre_bpm_hist: dict[str, dict[str, int]]
    key_route_counts: dict[str, int]
    sources: list[dict[str, str]]
    mean_transition_density: dict[str, float]
    bpm_center_estimate: dict[str, float]


@dataclass
class PolicyHint:
    """Solver overrides derived from aggregate setcraft stats."""

    bpm_center: float
    default_crossfade_secs: float


class TracklistSource(Protocol):
    """Parser backend — HTML/JSON adapters implement this."""

    def fetch_observations(self) -> Iterable[SetObservation]:
        ...


class JsonFixtureSource:
    """Load observations from a local JSON fixture (CI-safe)."""

    def __init__(self, path: Path) -> None:
        self.path = path

    def fetch_observations(self) -> Iterable[SetObservation]:
        data = json.loads(self.path.read_text(encoding="utf-8"))
        for row in data:
            yield SetObservation(
                source_url=row["source_url"],
                retrieved_at=row["retrieved_at"],
                genre=row["genre"],
                bpm_values=[float(x) for x in row.get("bpm_values", [])],
                key_routes=list(row.get("key_routes", [])),
                transition_density=float(row.get("transition_density", 0.0)),
            )


class CsvChartSource:
    """Load per-track chart rows from a local CSV export (CI-safe).

    Columns: ``source_url,retrieved_at,genre,bpm,key_from,key_to,transition_density``.
    Consecutive rows sharing ``source_url`` contribute BPM samples; routes use
    ``key_from->key_to`` when both are present.
    """

    def __init__(self, path: Path) -> None:
        self.path = path

    def fetch_observations(self) -> Iterable[SetObservation]:
        by_source: dict[str, SetObservation] = {}
        with self.path.open(newline="", encoding="utf-8") as fh:
            reader = csv.DictReader(fh)
            for row in reader:
                url = row["source_url"].strip()
                if url not in by_source:
                    by_source[url] = SetObservation(
                        source_url=url,
                        retrieved_at=row["retrieved_at"].strip(),
                        genre=row["genre"].strip(),
                        transition_density=float(row.get("transition_density") or 0.0),
                    )
                obs = by_source[url]
                bpm_raw = (row.get("bpm") or "").strip()
                if bpm_raw:
                    obs.bpm_values.append(float(bpm_raw))
                key_from = (row.get("key_from") or "").strip()
                key_to = (row.get("key_to") or "").strip()
                if key_from and key_to:
                    obs.key_routes.append(f"{key_from}->{key_to}")
                dens = (row.get("transition_density") or "").strip()
                if dens:
                    # Keep the last non-empty density for the set.
                    obs.transition_density = float(dens)
        yield from by_source.values()


def bpm_bucket(bpm: float) -> str:
    return str(int(bpm // 5 * 5))


def _median(values: list[float]) -> float:
    if not values:
        return 0.0
    return float(statistics.median(values))


def aggregate(observations: Iterable[SetObservation]) -> AggregateStats:
    genre_bpm_hist: dict[str, dict[str, int]] = {}
    key_route_counts: dict[str, int] = {}
    sources: list[dict[str, str]] = []
    density_acc: dict[str, list[float]] = {}
    bpm_acc: dict[str, list[float]] = {}

    for obs in observations:
        sources.append(
            {"source_url": obs.source_url, "retrieved_at": obs.retrieved_at, "genre": obs.genre}
        )
        hist = genre_bpm_hist.setdefault(obs.genre, {})
        bpm_acc.setdefault(obs.genre, [])
        density_acc.setdefault(obs.genre, []).append(obs.transition_density)
        for bpm in obs.bpm_values:
            b = bpm_bucket(bpm)
            hist[b] = hist.get(b, 0) + 1
            bpm_acc[obs.genre].append(bpm)
        for route in obs.key_routes:
            key_route_counts[route] = key_route_counts.get(route, 0) + 1

    mean_transition_density = {
        g: (sum(vals) / len(vals) if vals else 0.0) for g, vals in density_acc.items()
    }
    bpm_center_estimate = {g: _median(vals) for g, vals in bpm_acc.items()}

    return AggregateStats(
        genre_bpm_hist=genre_bpm_hist,
        key_route_counts=key_route_counts,
        sources=sources,
        mean_transition_density=mean_transition_density,
        bpm_center_estimate=bpm_center_estimate,
    )


def density_to_crossfade_secs(density: float) -> float:
    """Map transition density (transitions/min) to a default crossfade length.

    density ≥ 1.0 → 0.5s (quick cuts); density ≤ 0.4 → 12.0s (long blends);
    otherwise linear interpolate between those anchors.
    """
    if density >= 1.0:
        return 0.5
    if density <= 0.4:
        return 12.0
    # interpolate density 0.4..1.0 → crossfade 12.0..0.5
    t = (density - 0.4) / (1.0 - 0.4)
    return 12.0 + t * (0.5 - 12.0)


def policy_hints_from_stats(stats: AggregateStats) -> dict[str, dict[str, float]]:
    """Build genre → {bpm_center, default_crossfade_secs} from aggregates."""
    hints: dict[str, dict[str, float]] = {}
    genres = set(stats.bpm_center_estimate) | set(stats.mean_transition_density)
    for genre in sorted(genres):
        bpm = stats.bpm_center_estimate.get(genre, 120.0)
        dens = stats.mean_transition_density.get(genre, 0.5)
        hints[genre] = {
            "bpm_center": round(bpm, 2),
            "default_crossfade_secs": round(density_to_crossfade_secs(dens), 3),
        }
    return hints


def write_stats(path: Path, stats: AggregateStats) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "genre_bpm_hist": stats.genre_bpm_hist,
        "key_route_counts": stats.key_route_counts,
        "mean_transition_density": stats.mean_transition_density,
        "bpm_center_estimate": stats.bpm_center_estimate,
        "sources": stats.sources,
    }
    path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def write_policy_hints(path: Path, hints: dict[str, dict[str, float]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(hints, indent=2) + "\n", encoding="utf-8")


def merge_observations(*sources: TracklistSource) -> list[SetObservation]:
    """Collect observations from multiple offline sources."""
    out: list[SetObservation] = []
    for src in sources:
        out.extend(list(src.fetch_observations()))
    return out


def stats_as_dict(stats: AggregateStats) -> dict[str, Any]:
    return asdict(stats)
