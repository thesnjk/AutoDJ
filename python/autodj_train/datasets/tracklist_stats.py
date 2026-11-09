"""Tracklist / chart statistics ingest (aggregate only)."""

from __future__ import annotations

import json
from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Protocol


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
                bpm_values=list(row.get("bpm_values", [])),
                key_routes=list(row.get("key_routes", [])),
                transition_density=float(row.get("transition_density", 0.0)),
            )


def bpm_bucket(bpm: float) -> str:
    return str(int(bpm // 5 * 5))


def aggregate(observations: Iterable[SetObservation]) -> AggregateStats:
    genre_bpm_hist: dict[str, dict[str, int]] = {}
    key_route_counts: dict[str, int] = {}
    sources: list[dict[str, str]] = []
    for obs in observations:
        sources.append(
            {"source_url": obs.source_url, "retrieved_at": obs.retrieved_at, "genre": obs.genre}
        )
        hist = genre_bpm_hist.setdefault(obs.genre, {})
        for bpm in obs.bpm_values:
            b = bpm_bucket(bpm)
            hist[b] = hist.get(b, 0) + 1
        for route in obs.key_routes:
            key_route_counts[route] = key_route_counts.get(route, 0) + 1
    return AggregateStats(
        genre_bpm_hist=genre_bpm_hist,
        key_route_counts=key_route_counts,
        sources=sources,
    )


def write_stats(path: Path, stats: AggregateStats) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "genre_bpm_hist": stats.genre_bpm_hist,
        "key_route_counts": stats.key_route_counts,
        "sources": stats.sources,
    }
    path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
