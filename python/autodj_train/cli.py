"""CLI entrypoints for synth + tracklist aggregate."""

from __future__ import annotations

import argparse
from pathlib import Path

from autodj_train.datasets.synthetic import synthesize, write_jsonl
from autodj_train.datasets.tracklist_stats import (
    CsvChartSource,
    JsonFixtureSource,
    TracklistSource,
    aggregate,
    merge_observations,
    policy_hints_from_stats,
    write_policy_hints,
    write_stats,
)


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(prog="autodj-train")
    sub = parser.add_subparsers(dest="cmd", required=True)

    synth = sub.add_parser("synth", help="Write synthetic transition JSONL")
    synth.add_argument("--out", type=Path, required=True)
    synth.add_argument("--n", type=int, default=200)
    synth.add_argument("--seed", type=int, default=7)

    agg = sub.add_parser("aggregate-tracklists", help="Aggregate offline fixtures")
    agg.add_argument("--fixture", type=Path, action="append", default=[], help="JSON fixture (repeatable)")
    agg.add_argument("--csv", type=Path, action="append", default=[], help="CSV chart export (repeatable)")
    agg.add_argument("--out", type=Path, required=True, help="Aggregate stats JSON path")
    agg.add_argument("--hints-out", type=Path, default=None, help="Optional policy hints JSON")

    args = parser.parse_args(argv)
    if args.cmd == "synth":
        examples = synthesize(n=args.n, seed=args.seed)
        write_jsonl(args.out, examples)
        print(f"wrote {len(examples)} examples → {args.out}")
    elif args.cmd == "aggregate-tracklists":
        sources: list[TracklistSource] = []
        for path in args.fixture:
            sources.append(JsonFixtureSource(path))
        for path in args.csv:
            sources.append(CsvChartSource(path))
        if not sources:
            raise SystemExit("provide at least one --fixture or --csv")
        obs = merge_observations(*sources)
        stats = aggregate(obs)
        write_stats(args.out, stats)
        print(f"wrote aggregates → {args.out} ({len(stats.sources)} sources)")
        if args.hints_out is not None:
            hints = policy_hints_from_stats(stats)
            write_policy_hints(args.hints_out, hints)
            print(f"wrote policy hints → {args.hints_out} ({len(hints)} genres)")


if __name__ == "__main__":
    main()
