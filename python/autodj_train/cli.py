"""CLI entrypoints for synth, train, eval, predict, and tracklist aggregate."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from autodj_train.datasets.synthetic import (
    TransitionExample,
    read_jsonl,
    synthesize,
    write_jsonl,
)
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
from autodj_train.eval.metrics import evaluate_predictions
from autodj_train.models.softmax import SoftmaxLogReg

DEFAULT_GOLD = (
    Path(__file__).resolve().parent / "eval" / "gold" / "synthetic_holdout.jsonl"
)
DEFAULT_MODEL = (
    Path(__file__).resolve().parent / "models" / "artifacts" / "transition_model.json"
)
MIN_EVAL_ACCURACY = 0.90


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(prog="autodj-train")
    sub = parser.add_subparsers(dest="cmd", required=True)

    synth = sub.add_parser("synth", help="Write synthetic transition JSONL")
    synth.add_argument("--out", type=Path, required=True)
    synth.add_argument("--n", type=int, default=200)
    synth.add_argument("--seed", type=int, default=7)
    synth.add_argument("--balance", action="store_true", help="Balance class counts")

    train = sub.add_parser("train", help="Train SoftmaxLogReg and write model JSON")
    train.add_argument("--out", type=Path, required=True)
    train.add_argument("--n", type=int, default=2000)
    train.add_argument("--seed", type=int, default=7)
    train.add_argument("--jsonl", type=Path, default=None, help="Train from JSONL instead of synth")
    train.add_argument("--epochs", type=int, default=800)
    train.add_argument("--lr", type=float, default=0.35)

    ev = sub.add_parser("eval", help="Evaluate model on gold holdout")
    ev.add_argument("--model", type=Path, default=DEFAULT_MODEL)
    ev.add_argument("--gold", type=Path, default=DEFAULT_GOLD)
    ev.add_argument("--min-accuracy", type=float, default=MIN_EVAL_ACCURACY)

    pred = sub.add_parser("predict", help="Predict one transition class")
    pred.add_argument("--model", type=Path, default=DEFAULT_MODEL)
    pred.add_argument("--bpm-a", type=float, required=True)
    pred.add_argument("--bpm-b", type=float, required=True)
    pred.add_argument("--camelot-dist", type=int, required=True)
    pred.add_argument("--energy-delta-db", type=float, required=True)
    pred.add_argument("--genre", type=str, required=True)
    pred.add_argument("--centroid-a", type=float, default=1500.0)
    pred.add_argument("--centroid-b", type=float, default=1500.0)
    pred.add_argument("--bars-a", type=int, default=8)
    pred.add_argument("--bars-b", type=int, default=8)

    agg = sub.add_parser("aggregate-tracklists", help="Aggregate offline fixtures")
    agg.add_argument("--fixture", type=Path, action="append", default=[], help="JSON fixture")
    agg.add_argument("--csv", type=Path, action="append", default=[], help="CSV chart export")
    agg.add_argument("--out", type=Path, required=True, help="Aggregate stats JSON path")
    agg.add_argument("--hints-out", type=Path, default=None, help="Optional policy hints JSON")

    args = parser.parse_args(argv)
    if args.cmd == "synth":
        examples = synthesize(n=args.n, seed=args.seed, balance_labels=args.balance)
        write_jsonl(args.out, examples)
        print(f"wrote {len(examples)} examples → {args.out}")
    elif args.cmd == "train":
        if args.jsonl is not None:
            examples = read_jsonl(args.jsonl)
        else:
            examples = synthesize(n=args.n, seed=args.seed, balance_labels=True)
        model = SoftmaxLogReg(lr=args.lr, epochs=args.epochs, seed=args.seed)
        model.fit(examples)
        model.save(args.out)
        train_acc = model.accuracy(examples)
        print(f"wrote model → {args.out} (train_acc={train_acc:.4f}, n={len(examples)})")
    elif args.cmd == "eval":
        model = SoftmaxLogReg.load(args.model)
        supported = set(model.classes)
        gold = [ex for ex in read_jsonl(args.gold) if ex.label in supported]
        preds = model.predict(gold)
        report = evaluate_predictions(gold, preds)
        print(json.dumps(report, indent=2))
        acc = float(report["accuracy"])  # type: ignore[arg-type]
        if acc < args.min_accuracy:
            print(
                f"FAIL: accuracy {acc:.4f} < min {args.min_accuracy}",
                file=sys.stderr,
            )
            raise SystemExit(1)
        print(f"PASS: accuracy {acc:.4f} ≥ {args.min_accuracy}")
    elif args.cmd == "predict":
        model = SoftmaxLogReg.load(args.model)
        ex = TransitionExample(
            bpm_a=args.bpm_a,
            bpm_b=args.bpm_b,
            camelot_dist=args.camelot_dist,
            energy_delta_db=args.energy_delta_db,
            genre=args.genre,
            label="",
            spectral_centroid_hz_a=args.centroid_a,
            spectral_centroid_hz_b=args.centroid_b,
            bars_per_phrase_a=args.bars_a,
            bars_per_phrase_b=args.bars_b,
        )
        print(model.predict_one(ex))
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
