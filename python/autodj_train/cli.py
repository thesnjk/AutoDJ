"""CLI entrypoints for synth + tracklist aggregate."""

from __future__ import annotations

import argparse
from pathlib import Path

from autodj_train.datasets.synthetic import synthesize, write_jsonl
from autodj_train.datasets.tracklist_stats import JsonFixtureSource, aggregate, write_stats


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(prog="autodj-train")
    sub = parser.add_subparsers(dest="cmd", required=True)

    synth = sub.add_parser("synth", help="Write synthetic transition JSONL")
    synth.add_argument("--out", type=Path, required=True)
    synth.add_argument("--n", type=int, default=200)
    synth.add_argument("--seed", type=int, default=7)

    agg = sub.add_parser("aggregate-tracklists", help="Aggregate fixture observations")
    agg.add_argument("--fixture", type=Path, required=True)
    agg.add_argument("--out", type=Path, required=True)

    args = parser.parse_args(argv)
    if args.cmd == "synth":
        examples = synthesize(n=args.n, seed=args.seed)
        write_jsonl(args.out, examples)
        print(f"wrote {len(examples)} examples → {args.out}")
    elif args.cmd == "aggregate-tracklists":
        source = JsonFixtureSource(args.fixture)
        stats = aggregate(source.fetch_observations())
        write_stats(args.out, stats)
        print(f"wrote aggregates → {args.out} ({len(stats.sources)} sources)")


if __name__ == "__main__":
    main()
