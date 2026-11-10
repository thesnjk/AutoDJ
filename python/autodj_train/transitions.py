"""Transition class taxonomy shared with Rust (`TransitionClass`)."""

from __future__ import annotations

from enum import Enum


class TransitionClass(str, Enum):
    LINEAR_CROSSFADE = "linear_crossfade"
    EQ_SWAP = "eq_swap"
    FILTER_SWEEP = "filter_sweep"
    ECHO_OUT = "echo_out"
    TREMOLO = "tremolo"
    HARD_CUT = "hard_cut"


ALL_CLASSES = list(TransitionClass)
