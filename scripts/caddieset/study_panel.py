"""Does the six-checkpoint panel predict ball flight? The gate for M8-PAIR.

Usage:
    python scripts/caddieset/study_panel.py [--quiet]

Needs the `research` extra (numpy, scikit-learn). Reads `data/reference/caddieset/shots.jsonl`.

### The question

`golfdb_v1.json` can say a swing is *unusual* against 122 tour players. Nothing in this repo can
say a swing is *bad*, because GolfDB carries no ball flight (ADR-012, and
`contracts/comparison.py::NO_LAUNCH_MONITOR_POPULATION` says the same in code). CaddieSet is the
first corpus here with mechanics and outcome on the same row, so it is the first chance to ask
whether the six things we score are things that matter.

### Why leave-one-golfer-out, and why per-fold

Eight golfers, and the outcome base rate swings from 18% to 79% between them. A model given all
1,757 rows will happily learn *which golfer this is* and score beautifully while knowing nothing
about golf. `LeaveOneGroupOut` on `golfer_id` is therefore not a refinement, it is the only
honest split, and it is the same reasoning `bakeoff_sample` uses to spread clips across source
videos rather than take the first N.

Per-fold AUC is also the *right* metric rather than a nuisance, and this is worth being explicit
about: AUC inside one golfer's shots asks "do these features rank this golfer's good shots above
their bad ones", which is precisely the coaching question. Pooling across folds would let a
between-golfer difference in base rate masquerade as skill at ranking.

A fold is **skipped, not scored**, when it cannot support the claim: fewer than
`_MIN_FOLD_SAMPLES` shots, or fewer than `_MIN_FOLD_MINORITY` in either class. Reporting an AUC
over four negatives would be the confident-voice-over-noise failure this repo exists to avoid
(`contracts/baseline.py` encodes the same instinct as sample floors).

### What this study may and may not conclude

It may conclude *which* mechanical facts carry signal, and in which direction. It may **not**
export a number. CaddieSet's joint metrics are CaddieSet's own definitions computed by
CaddieSet's own pipeline, not our `metric_definitions_version: 3` measurements, and ADR-012 §4 is
the rule: a band is only comparable to a swing measured the same way. Nothing here writes to
`ranges.json`, and nothing here should be quoted as one.
"""

from __future__ import annotations

import sys
from typing import Any

import common
import numpy as np
from sklearn.ensemble import HistGradientBoostingClassifier
from sklearn.impute import SimpleImputer
from sklearn.linear_model import LogisticRegression, Ridge
from sklearn.metrics import roc_auc_score
from sklearn.model_selection import LeaveOneGroupOut, StratifiedKFold
from sklearn.pipeline import Pipeline
from sklearn.preprocessing import StandardScaler

# A fold below these cannot support an AUC. 25 shots with at least 5 in the minority class is
# already generous — it is the floor at which the number stops being an anecdote, not the point
# at which it becomes precise.
_MIN_FOLD_SAMPLES = 25
_MIN_FOLD_MINORITY = 5

# Robust-z clip, fit on the training fold only. The corpus has pose failures that put 33
# shoulder-widths of head travel in a cell (`5-HEAD-LOC` runs to 15.65 against a median of
# -0.25), and a single one of those moves a standardised coefficient more than a hundred real
# swings do. Clipping rather than dropping keeps the row: the other 39 columns are still evidence.
_CLIP_Z = 5.0

# CaddieSet's own outcome definitions, from the paper: a straight start is within 6 degrees, a
# desirable spin axis within 10. Both are read from the launch monitor, so they are the closest
# thing to ground truth in this project.
_STRAIGHT_DEG = 6.0
_GOOD_AXIS_DEG = 10.0

# Phase indices into CaddieSet's `{phase}-{METRIC}` columns. Confirmed against the data rather
# than assumed: `HIP-SHIFTED` runs negative through phases 1-4 (hips loading away from target)
# and turns positive by phase 5, which is only true if 3 is the top and 5 is impact. `WEIGHT-
# SHIFT` at phase 6 sits at 94%, and `FINISH-ANGLE` exists only at 7. The physics agrees with
# `common.PHASE_NAMES`.
_TOP = 3
_IMPACT = 5

# Which CaddieSet columns stand in for each of our checkpoints. These are *conceptual* analogues:
# theirs is a per-phase position, ours is usually a range or a difference, so the derived features
# below are built to close the gap where they can. `tempo` has no entry at all and that is the
# finding, not an oversight — the CSV ships no frame indices, so no ratio of durations can be
# recovered from it. It is also the one checkpoint currently failing on the golfer's own swings.
_PANEL_ANALOGUES: dict[str, tuple[str, ...]] = {
    "tempo": (),
    "head_sway": ("head_sway_range",),
    "finish_balance": ("7-FINISH-ANGLE",),
    "hip_sway": ("hip_sway_range",),
    "hip_shift_at_top": (f"{_TOP}-HIP-SHIFTED",),
    "head_stays_back": (
        f"{_IMPACT}-HEAD-LOC",
        f"{_IMPACT}-HIP-HANGING-BACK",
        f"{_IMPACT}-SHOULDER-HANGING-BACK",
    ),
}


def _derived(metrics: dict[str, float]) -> dict[str, float]:
    """Range features, so a per-phase position can stand in for one of our spans.

    `head_sway_norm` and `hip_sway_norm` are travel *across* the swing, not a position at one
    instant, so comparing them to a single CaddieSet column would understate them. Built over the
    phases each of ours is measured across: head sway from takeaway to impact, hip sway over the
    whole swing.
    """
    out: dict[str, float] = {}
    head = [metrics[f"{p}-HEAD-LOC"] for p in range(1, _IMPACT + 1) if f"{p}-HEAD-LOC" in metrics]
    if len(head) >= 3:
        out["head_sway_range"] = max(head) - min(head)
    hips = [metrics[f"{p}-HIP-SHIFTED"] for p in range(1, 8) if f"{p}-HIP-SHIFTED" in metrics]
    if len(hips) >= 4:
        out["hip_sway_range"] = max(hips) - min(hips)
    return out


def build_matrix(
    shots: list[dict[str, Any]],
) -> tuple[np.ndarray, list[str], dict[str, np.ndarray]]:
    """Feature matrix over face-on shots, with NaN for absent cells.

    NaN rather than an imputed value at this stage, so imputation can happen inside the fold and
    cannot leak the held-out golfer's median into the training set.
    """
    enriched = [dict(s["metrics"], **_derived(s["metrics"])) for s in shots]
    columns = sorted({c for row in enriched for c in row})
    matrix = np.full((len(shots), len(columns)), np.nan)
    for i, row in enumerate(enriched):
        for j, column in enumerate(columns):
            if column in row:
                matrix[i, j] = row[column]

    meta = {
        "golfer": np.array([s["golfer_id"] for s in shots]),
        "club": np.array([s["club"] for s in shots]),
        "direction": np.array([s["outcome"]["DirectionAngle"] for s in shots]),
        "spin_axis": np.array([s["outcome"]["SpinAxis"] for s in shots]),
        "carry": np.array([s["outcome"]["Carry"] for s in shots]),
    }
    return matrix, columns, meta


def _clip_to_train(train: np.ndarray, other: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """Robust-z clip both arrays to the *training* fold's median and MAD."""
    median = np.nanmedian(train, axis=0)
    mad = np.nanmedian(np.abs(train - median), axis=0)
    # 1.4826 makes MAD a consistent estimator of sigma for normal data. Where MAD is zero the
    # column is constant on this fold and clipping is a no-op rather than a divide by zero.
    scale = np.where(mad > 0, mad * 1.4826, np.inf)
    low, high = median - _CLIP_Z * scale, median + _CLIP_Z * scale
    return np.clip(train, low, high), np.clip(other, low, high)


def _classifier(kind: str, c: float = 0.1) -> Pipeline | HistGradientBoostingClassifier:
    if kind == "boosted":
        # Handles NaN natively and needs no scaling. Included as the non-linear comparison: if it
        # cannot beat the linear model, non-linearity is not what is missing.
        return HistGradientBoostingClassifier(max_iter=200, max_depth=3, random_state=0)
    return Pipeline(
        [
            ("impute", SimpleImputer(strategy="median")),
            ("scale", StandardScaler()),
            ("model", LogisticRegression(C=c, max_iter=2000)),
        ]
    )


def logo_auc(
    matrix: np.ndarray, target: np.ndarray, groups: np.ndarray, kind: str = "linear"
) -> tuple[list[tuple[str, int, float, float]], list[tuple[str, str]]]:
    """Leave-one-golfer-out AUC. Returns (scored folds, skipped folds with a reason)."""
    scored: list[tuple[str, int, float, float]] = []
    skipped: list[tuple[str, str]] = []

    for train_idx, test_idx in LeaveOneGroupOut().split(matrix, target, groups):
        held = groups[test_idx][0]
        y_test = target[test_idx]
        minority = min(int(y_test.sum()), int((~y_test.astype(bool)).sum()))
        if len(test_idx) < _MIN_FOLD_SAMPLES:
            skipped.append((held, f"{len(test_idx)} shots < {_MIN_FOLD_SAMPLES}"))
            continue
        if minority < _MIN_FOLD_MINORITY:
            skipped.append((held, f"{minority} in the minority class < {_MIN_FOLD_MINORITY}"))
            continue

        x_train, x_test = _clip_to_train(matrix[train_idx], matrix[test_idx])
        model = _classifier(kind)
        model.fit(x_train, target[train_idx])
        probability = model.predict_proba(x_test)[:, 1]
        auc = roc_auc_score(y_test, probability)
        scored.append((held, len(test_idx), float(y_test.mean()), auc))

    return scored, skipped


def _summary(scored: list[tuple[str, int, float, float]]) -> float:
    return float(np.mean([auc for *_, auc in scored])) if scored else float("nan")


def center_within_golfer(matrix: np.ndarray, groups: np.ndarray) -> np.ndarray:
    """Subtract each golfer's own mean from every feature.

    With eight subjects, a coefficient that is stable across folds is **not** evidence of a
    mechanism: seven of eight golfers happening to stand wider and also happening to slice less
    produces exactly the same stable coefficient as a real effect, and the sign-stability column
    cannot tell them apart. Centering each golfer on themselves deletes every between-golfer
    difference and leaves only "when *this* golfer does more of this than they usually do, what
    happens to the ball" — which is the mechanism, and the only version of it a coach could act on.

    Worth knowing what this costs in deployment: centering needs the golfer's own mean, so any
    model fitted on centered features requires a personal baseline before it can score a swing at
    all. That is career mode's `PersonalBaseline`, and this is the study that says why it is a
    prerequisite rather than a nicety.
    """
    centered = matrix.copy()
    for golfer in set(groups):
        index = np.flatnonzero(groups == golfer)
        with np.errstate(invalid="ignore"):
            centered[index] -= np.nanmean(matrix[index], axis=0)
    return centered


def within_golfer_auc(
    matrix: np.ndarray, target: np.ndarray, groups: np.ndarray, kind: str = "linear"
) -> list[tuple[str, int, float, float]]:
    """Train and test on the *same* golfer, 5-fold. The control that reads the negative result.

    Leave-one-golfer-out failing has two very different explanations and they call for opposite
    products. Either these features carry no information about ball flight at all, or they carry
    it in a way that is **specific to the golfer** — the same mechanical measurement meaning
    different things for two different bodies and two different swings. Only a within-golfer fit
    can tell those apart, so it is run alongside rather than instead.

    Out-of-fold probabilities are pooled and scored once per golfer, which is the honest way to
    get one AUC out of k folds without averaging over folds too small to rank anything.
    """
    scored: list[tuple[str, int, float, float]] = []
    for golfer in sorted(set(groups), key=str):
        index = np.flatnonzero(groups == golfer)
        y = target[index]
        minority = min(int(y.sum()), int(len(y) - y.sum()))
        if len(index) < _MIN_FOLD_SAMPLES or minority < 2 * _MIN_FOLD_MINORITY:
            continue
        splits = min(5, minority)
        probability = np.full(len(index), np.nan)
        folds = StratifiedKFold(n_splits=splits, shuffle=True, random_state=0)
        for train_idx, test_idx in folds.split(matrix[index], y):
            x_train, x_test = _clip_to_train(matrix[index][train_idx], matrix[index][test_idx])
            model = _classifier(kind)
            model.fit(x_train, y[train_idx])
            probability[test_idx] = model.predict_proba(x_test)[:, 1]
        scored.append((str(golfer), len(index), float(y.mean()), roc_auc_score(y, probability)))
    return scored


def report_target(
    name: str,
    matrix: np.ndarray,
    columns: list[str],
    target: np.ndarray,
    meta: dict[str, np.ndarray],
) -> dict[str, float]:
    """Run every feature set against one target and print the comparison."""
    groups = meta["golfer"]
    club_onehot = np.array(
        [[1.0 if c == club else 0.0 for club in sorted(set(meta["club"]))] for c in meta["club"]]
    )
    panel_columns = [c for names in _PANEL_ANALOGUES.values() for c in names]
    panel_idx = [columns.index(c) for c in panel_columns if c in columns]

    print(f"\n{'=' * 78}")
    print(f"{name}   (base rate {target.mean():.1%} over {len(target)} face-on shots)")
    print("=" * 78)

    centered = center_within_golfer(matrix, groups)
    feature_sets: list[tuple[str, np.ndarray, str]] = [
        ("club only (baseline)", club_onehot, "linear"),
        (f"panel analogues ({len(panel_idx)} features)", matrix[:, panel_idx], "linear"),
        (f"all joint columns ({len(columns)})", matrix, "linear"),
        (f"all joint columns ({len(columns)}), boosted", matrix, "boosted"),
        (f"all joint columns ({len(columns)}), centered per golfer", centered, "linear"),
    ]

    results: dict[str, float] = {}
    for label, features, kind in feature_sets:
        scored, skipped = logo_auc(features, target, groups, kind)
        mean_auc = _summary(scored)
        results[label] = mean_auc
        print(f"\n  {label}")
        print(f"    mean per-golfer AUC: {mean_auc:.3f}   ({len(scored)} folds scored)")
        for held, n, rate, auc in sorted(scored, key=lambda r: -r[3]):
            print(f"      golfer {held}: n={n:4d}  base {rate:5.1%}  AUC {auc:.3f}")
        for held, reason in skipped:
            print(f"      golfer {held}: SKIPPED — {reason}")

    # The control. Same features, same model, but never asked to cross a golfer boundary.
    within = within_golfer_auc(matrix, target, groups)
    results["within-golfer (control, not a baseline)"] = _summary(within)
    print(f"\n  within-golfer, all {len(columns)} columns (control — trains on the same golfer)")
    print(f"    mean AUC: {_summary(within):.3f}   ({len(within)} golfers scored)")
    for held, n, rate, auc in sorted(within, key=lambda r: -r[3]):
        print(f"      golfer {held}: n={n:4d}  base {rate:5.1%}  AUC {auc:.3f}")

    return results


def sensitivity(
    matrix: np.ndarray, target: np.ndarray, groups: np.ndarray, label: str
) -> None:
    """Does the verdict survive a different regularisation strength?

    One C is a choice, and a conclusion that only holds at one C is a conclusion about the choice.
    Swept rather than tuned: the point is that the answer does not move, not to find the best
    number — tuning against the same folds the verdict is read from would be the fishing this
    check exists to rule out.
    """
    print(f"\n  regularisation sweep, {label} (mean per-golfer AUC, leave-one-golfer-out):")
    for c in (0.01, 0.1, 1.0, 10.0):
        scored: list[tuple[str, int, float, float]] = []
        for train_idx, test_idx in LeaveOneGroupOut().split(matrix, target, groups):
            y_test = target[test_idx]
            minority = min(int(y_test.sum()), int(len(y_test) - y_test.sum()))
            if len(test_idx) < _MIN_FOLD_SAMPLES or minority < _MIN_FOLD_MINORITY:
                continue
            x_train, x_test = _clip_to_train(matrix[train_idx], matrix[test_idx])
            model = _classifier("linear", c=c)
            model.fit(x_train, target[train_idx])
            auc = roc_auc_score(y_test, model.predict_proba(x_test)[:, 1])
            scored.append((groups[test_idx][0], len(test_idx), float(y_test.mean()), auc))
        print(f"    C={c:<6g} {_summary(scored):.3f}")


def report_coefficients(
    matrix: np.ndarray,
    columns: list[str],
    target: np.ndarray,
    meta: dict[str, np.ndarray],
    label: str,
) -> dict[str, float]:
    """Standardised logistic coefficients averaged over the folds, largest magnitude first."""
    groups = meta["golfer"]
    coefficients: list[np.ndarray] = []
    for train_idx, _ in LeaveOneGroupOut().split(matrix, target, groups):
        x_train, _unused = _clip_to_train(matrix[train_idx], matrix[train_idx])
        model = _classifier("linear")
        model.fit(x_train, target[train_idx])
        coefficients.append(model.named_steps["model"].coef_[0])

    stacked = np.vstack(coefficients)
    mean = stacked.mean(axis=0)
    # A coefficient that flips sign between folds is a coefficient fitted to one golfer.
    stable = (np.sign(stacked) == np.sign(mean)).mean(axis=0)
    order = np.argsort(-np.abs(mean))

    print(f"\n  {label} (mean over folds, sign-stability across folds):")
    for j in order[:12]:
        flag = "" if stable[j] >= 0.75 else "   <- sign unstable"
        print(f"    {columns[j]:26s} {mean[j]:+7.3f}   stable {stable[j]:4.0%}{flag}")
    return {columns[j]: float(mean[j]) for j in order}


def report_panel_verdict(
    raw: dict[str, float], centered: dict[str, float], columns: list[str]
) -> None:
    """Per checkpoint: is there an analogue here, and did it carry weight?

    Both rankings are shown because they answer different questions. `raw` includes between-golfer
    differences, `centered` is the within-golfer mechanism. A feature that ranks high raw and low
    centered is a trait of the golfers in this corpus, not a lever any of them could pull.
    """
    raw_rank = sorted(raw, key=lambda c: -abs(raw[c]))
    centered_rank = sorted(centered, key=lambda c: -abs(centered[c]))
    print(f"\n{'=' * 78}\nPER-CHECKPOINT VERDICT\n{'=' * 78}")
    print(f"  {'':26s} {'raw':>18s}  {'centered per golfer':>20s}")
    for checkpoint, analogues in _PANEL_ANALOGUES.items():
        if not analogues:
            print(f"\n  {checkpoint}: NO ANALOGUE IN THIS CORPUS")
            print("      CaddieSet ships no frame indices, so no ratio of durations survives.")
            continue
        print(f"\n  {checkpoint}:")
        for name in analogues:
            if name not in columns:
                print(f"      {name:26s} absent from the face-on subset")
                continue
            print(
                f"      {name:26s} {raw[name]:+7.3f} #{raw_rank.index(name) + 1:<3d}"
                f"   {centered[name]:+9.3f} #{centered_rank.index(name) + 1:<3d}"
            )


def carry_signal(matrix: np.ndarray, columns: list[str], meta: dict[str, np.ndarray]) -> None:
    """Can mechanics predict *distance*, once the club and the golfer are taken out of it?

    Raw carry is mostly club: driver averages 169 m and a 9-iron 118. Standardising within each
    (golfer, club) cell turns the target into "long for you, with that club", which is the only
    version of the question mechanics could answer.
    """
    cells: dict[tuple[str, str], list[int]] = {}
    for i, (golfer, club) in enumerate(zip(meta["golfer"], meta["club"], strict=True)):
        cells.setdefault((golfer, club), []).append(i)

    target = np.full(len(meta["carry"]), np.nan)
    for indices in cells.values():
        if len(indices) < 20:
            continue
        values = meta["carry"][indices]
        spread = values.std()
        if spread > 0:
            target[indices] = (values - values.mean()) / spread

    usable = ~np.isnan(target)
    print(f"\n{'=' * 78}\nCARRY, standardised within (golfer, club)\n{'=' * 78}")
    print(f"  usable shots: {usable.sum()}/{len(target)} in cells of >=20")

    scores = []
    for train_idx, test_idx in LeaveOneGroupOut().split(
        matrix[usable], target[usable], meta["golfer"][usable]
    ):
        if len(test_idx) < _MIN_FOLD_SAMPLES:
            continue
        x_train, x_test = _clip_to_train(matrix[usable][train_idx], matrix[usable][test_idx])
        model = Pipeline(
            [
                ("impute", SimpleImputer(strategy="median")),
                ("scale", StandardScaler()),
                ("model", Ridge(alpha=10.0)),
            ]
        )
        model.fit(x_train, target[usable][train_idx])
        prediction = model.predict(x_test)
        actual = target[usable][test_idx]
        # R^2 against the held-out golfer's own mean, so a model that predicts the average shot
        # scores 0 rather than being flattered by between-golfer spread it never saw.
        residual = float(((actual - prediction) ** 2).sum())
        total = float(((actual - actual.mean()) ** 2).sum())
        scores.append(1.0 - residual / total if total > 0 else float("nan"))

    finite = [s for s in scores if np.isfinite(s)]
    print(f"  mean held-out R^2 over {len(finite)} folds: {np.mean(finite):+.3f}")
    print("  (negative means the model does worse than predicting that golfer's average shot)")


def main(argv: list[str]) -> int:
    if argv and argv[0] in {"-h", "--help"}:
        print("usage: python scripts/caddieset/study_panel.py [--quiet]", file=sys.stderr)
        return 2

    shots = common.face_on(common.load_shots())
    matrix, columns, meta = build_matrix(shots)
    print(
        f"face-on shots: {len(shots)}   features: {len(columns)}   "
        f"golfers: {len(set(meta['golfer']))}"
    )

    straight = (np.abs(meta["direction"]) <= _STRAIGHT_DEG).astype(int)
    good_axis = (np.abs(meta["spin_axis"]) <= _GOOD_AXIS_DEG).astype(int)

    straight_results = report_target(
        f"STRAIGHT START  (|DirectionAngle| <= {_STRAIGHT_DEG:g} deg)",
        matrix,
        columns,
        straight,
        meta,
    )
    axis_results = report_target(
        f"DESIRABLE SPIN AXIS  (|SpinAxis| <= {_GOOD_AXIS_DEG:g} deg)",
        matrix,
        columns,
        good_axis,
        meta,
    )

    print(f"\n{'=' * 78}\nROBUSTNESS\n{'=' * 78}")
    sensitivity(matrix, straight, meta["golfer"], "straight start")
    sensitivity(matrix, good_axis, meta["golfer"], "spin axis")

    print(f"\n{'=' * 78}\nWHICH FEATURES, FOR SPIN AXIS\n{'=' * 78}")
    raw_coefficients = report_coefficients(
        matrix, columns, good_axis, meta, "standardised coefficients, raw"
    )
    centered_coefficients = report_coefficients(
        center_within_golfer(matrix, meta["golfer"]),
        columns,
        good_axis,
        meta,
        "standardised coefficients, centered per golfer (the within-golfer mechanism)",
    )
    report_panel_verdict(raw_coefficients, centered_coefficients, columns)

    carry_signal(matrix, columns, meta)

    print(f"\n{'=' * 78}\nGATE\n{'=' * 78}")
    for name, results in (("straight start", straight_results), ("spin axis", axis_results)):
        baseline = next(v for k, v in results.items() if k.startswith("club only"))
        control = results["within-golfer (control, not a baseline)"]
        # The control is excluded from `best` deliberately: it is fitted on the golfer it is
        # scored on, so treating it as a candidate would let the gate pass on a model that cannot
        # be handed a golfer it has not already seen.
        best_label, best = max(
            (
                (k, v)
                for k, v in results.items()
                if not k.startswith(("club only", "within-golfer"))
            ),
            key=lambda kv: kv[1],
        )
        verdict = "PASS" if best > baseline + 0.02 else "FAIL"
        print(
            f"  {name:16s} transfer {best:.3f} ({best_label})"
            f"  vs club-only {baseline:.3f}  -> {verdict}"
        )
        print(f"  {'':16s} within-golfer control {control:.3f}")
    print("\n  A pass means mechanics rank a golfer's own shots better than club choice alone,")
    print("  when trained on golfers other than that one.")
    print("  Nothing here may be written into ranges.json (ADR-012 §4).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
