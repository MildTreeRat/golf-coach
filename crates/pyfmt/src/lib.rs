//! CPython's rounding, float formatting and dict ordering, in one place. [M22 P3]
//!
//! ADR-032 §3's three portability edges. They are one module rather than three because they are
//! one problem: all three reach the **strings** `docs/CONFORMANCE.md` §3 compares exactly, so
//! getting any of them wrong fails a vector on a sentence a golfer was shown rather than on a
//! number, and the failure looks nothing like its cause. M22 P3 built it before the engine had a
//! line in it, so the edges were solved once instead of rediscovered at each of the seven call
//! sites downstream.
//!
//! # A crate of its own since M34 P1
//!
//! It was `crates/analysis/src/pyfmt.rs`, and it moved without a function changing. What moved it
//! is that the callers are no longer all inside `analysis`. `crates/feedback` may not import
//! `analysis` (ADR-008 as a cargo edge), so for one fallback sentence it had re-spelled
//! [`percent`] beside a test holding the two spellings to one rule. `crates/screen` is in the
//! same position with a whole parser's worth of `%g`, `.Nf` and banker's rounding, and a second
//! copy of this module is exactly the drift it exists to prevent. Depending on nothing of ours
//! puts it below every crate that has to say what Python said. No crate re-exports it, so each
//! one names it by the same path.
//!
//! The gate is `spec/vectors/format/`, run by `tests/format.rs`: every case recorded from CPython
//! itself, compared **exactly** with no tolerance at all, because nothing in this module computes
//! a measurement — a rounding rule is either the same rule or it is a different one.
//!
//! # The load-bearing discovery, and what it collapses
//!
//! Rust's `{:.N}` and `{:.Ne}` are not the quick approximations they look like. `core::fmt`'s
//! exact path (`flt2dec`) generates digits from the f64's **exact** binary value and breaks a tie
//! **to even** — which is precisely the rule CPython applies through `_Py_dg_dtoa`. So two of the
//! three edges reduce to Rust's own formatter:
//!
//! - [`round_to`] is `format!("{x:.n$}")` parsed back, and *not* `(x * 10f64.powi(n)).round() / …`.
//!   The scaled version is wrong on ordinary values rather than on contrived ones, and the rate is
//!   worth knowing: over 200,000 three-decimal values in `[0, 10)` it disagrees with CPython on
//!   **4.6%** of them at two places, even when the scaling is given ties-to-even as well. The
//!   multiply is what breaks it — `0.215` is exactly `0.21499999999999999667`, so it rounds *down*
//!   to `0.21`, but `0.215 * 100.0` is `21.5` on the nose and rounds up to `0.22`.
//! - [`g`] takes its mantissa and exponent from `{:.5e}`, which puts the six-significant-digit
//!   rounding in the formatter where it is already right.
//!
//! What does **not** reduce to it is the two places Rust's text differs from Python's: a NaN prints
//! as `NaN` here and `nan` there, and Rust has no `%g` at all. Both are handled below and both are
//! in the vector table, because a NaN reaching a sentence is a bug either way and the two
//! languages should at least agree about which bug.
//!
//! # A fourth edge landed here in P8, and it is not a string
//!
//! [`hypot`] is CPython's `math.hypot`, which Rust's std does not have in the three-argument form the
//! ball-flight integrator calls it in. It is here rather than in `analysis::flight` for the reason this
//! module exists at all — one home, so the next call site does not re-solve it differently — and its
//! own doc carries the measurement. What makes it belong beside the other three is that it is the
//! same *kind* of thing: a CPython semantic Rust's standard library does not reproduce, whose
//! difference reaches a value `docs/CONFORMANCE.md` §3 compares **exactly**. Where the first three
//! reach a sentence, this one reaches a **bool** — `AeroCoefficients::clamped`, at a spin the solve
//! constructs to sit exactly on the coefficient table's last row.
//!
//! # The screen parser's edges, from M34 P2
//!
//! The frozen screen parser leans on CPython in places the engine never did, and each is the same
//! kind of thing as the four above: a CPython semantic whose Rust look-alike is close enough to
//! pass a glance and differs where it is measured. [`str_repr`] is `{text!r}` inside a warning;
//! [`floor_div`] is `_Cell.text`'s line bucket; [`upper`], [`split`], [`strip`] and
//! [`strip_space`] are its Unicode string handling, all four standing on [`is_space`]; and [`sum`]
//! is `_score`'s mean, compensated since CPython 3.12. Each is gated by its own table in
//! `spec/vectors/format/`, recorded from the interpreter that recorded everything else, and each
//! doc says what the Rust call it replaces gets wrong. `difflib`'s `ratio` is the sixth of the
//! parser's edges and lives in `crates/screen`, which is its only caller.
//!
//! # The many-shot layer's, from M36 P4
//!
//! Two more, before any caller exists, for the career aggregates, the stores and the report
//! verbs: [`general`] is `:.Ng` at a precision other than `%g`'s six, and [`lower`] is
//! `str.lower()`. Each is gated by a table of its own (`general_precision`, `lower`). The third
//! edge that layer found is pydantic's `datetime`, which is `contracts::time::Timestamp`'s, because
//! it is a value a contract carries rather than a string a sentence prints.

/// Python's `round(x)` — the one-argument form, which is half-to-**even** where Rust's
/// [`f64::round`] is half-away-from-zero.
///
/// `2.5` goes to `2` and `3.5` goes to `4`. CPython does this explicitly: it calls C's `round`
/// and then, if the value sat exactly on the half, replaces the answer with `2.0 * round(x / 2.0)`.
/// [`f64::round_ties_even`] is the same rule in one call.
///
/// The seventeen frame-index sites are why this matters more than it looks. A checkpoint measured
/// one frame late is not a rounding difference in the output, it is a different measurement, and
/// every score after it inherits the error — `analysis/alignment.py` alone rounds a float frame
/// count in ten anchor fallbacks.
pub fn round_half_even(x: f64) -> f64 {
    x.round_ties_even()
}

/// Python's `round(x)` where the result indexes a frame, with the cast made in one place.
///
/// Python's `round` returns an unbounded `int` and this returns an `i64`, which is a real
/// narrowing and is safe here for a reason rather than by luck: every caller is rounding a frame
/// count or a sample offset, and a clip long enough to overflow `i64` would have exhausted the
/// disk several times over. A saturating cast is what Rust's `as` already does for f64 -> i64, so
/// a non-finite input lands on a bound instead of wrapping.
pub fn round_index(x: f64) -> i64 {
    round_half_even(x) as i64
}

/// Python's `round(x, ndigits)` — decimal-aware, and **not** a scaled [`f64::round`].
///
/// `round(x, n)` in CPython rounds the decimal expansion of the *exact* binary value to `n` places,
/// ties to even, and converts back. Rust's `{:.n$}` does the same thing by the same rule (see the
/// module doc), so formatting and reparsing is the port rather than a shortcut around one. The
/// scaled form is the trap: it rounds twice, and the first rounding is `x * 10f64.powi(n)`, which
/// has already lost the value it was meant to inspect.
///
/// Non-finite input returns unchanged, which is what Python does — `round(inf, 4)` is `inf`. The
/// early return is not only for correctness: Rust would format a NaN as the string `NaN`, which
/// round-trips, and an infinity as `inf`, which also round-trips, so this is the cheaper path
/// rather than a rescue.
pub fn round_to(x: f64, ndigits: usize) -> f64 {
    if !x.is_finite() {
        return x;
    }
    format!("{x:.ndigits$}")
        .parse()
        .expect("a fixed-point rendering of a finite f64 always parses back")
}

/// Python's `f"{x:.Nf}"` — the seventy-three `.Nf` sites that reach a compared sentence.
///
/// Rust's `{:.N}` agrees with this on every finite value, ties included, and the vector table is
/// what says so rather than this comment. The one divergence is a NaN: Rust writes `NaN` and
/// Python writes `nan`, so that case is rewritten here. An infinity needs no help — both write
/// `inf` and `-inf` — and a negative zero needs none either, because both keep the sign and write
/// `-0.00`, which is the detail a port that normalizes it would get wrong in prose only.
pub fn fixed(x: f64, precision: usize) -> String {
    if x.is_nan() {
        return "nan".to_string();
    }
    format!("{x:.precision$}")
}

/// Python's `f"{x:+.Nf}"` — [`fixed`] with the sign always written. [M22 P6]
///
/// One site, and it is a correction a reader has to be able to see the direction of:
/// `engine.py::_anchored_on_strike` writes `impact moved {moved:+d} frames ({seconds:+.3f}s)`, and
/// an unsigned `0.125s` there does not say whether pose had impact early or late.
///
/// **Derived from [`fixed`] rather than given a table of its own, and that is the point.** CPython
/// renders the digits first and inserts the sign after, so the only thing `+` adds to `.Nf` is a
/// leading character on a rendering that has none — which means `spec/vectors/format/fixed.json`'s
/// cases already gate every digit this function can produce, and a second table would be a second
/// thing to regenerate for no new answer. The three cases where that composition is not obvious
/// are all pinned below: `-0.0` renders as `-0.000` and must **not** gain a `+`, a NaN is `+nan`
/// because CPython signs the word too, and `-inf` keeps its own sign.
///
/// Testing `starts_with('-')` rather than `x < 0.0` is what makes the negative-zero case fall out
/// instead of needing a branch: `x < 0.0` is false for `-0.0`, and would write `+-0.000`.
pub fn signed_fixed(x: f64, precision: usize) -> String {
    let rendered = fixed(x, precision);
    if rendered.starts_with('-') {
        rendered
    } else {
        format!("+{rendered}")
    }
}

/// Python's `f"{x:.N%}"` — a fraction as a percentage. [M22 P6]
///
/// One site, and it is currently unreachable: `feedback/rules.py::_tip_for` falls back to
/// `f"{name}: score {score:.0%}."` when a `CheckpointScore` carries no `message`, and every scored
/// checkpoint on all 21 vectors carries one. So it is ported for the reason the Python's fallback
/// exists at all — a checkpoint whose evaluator forgets its sentence should still say something
/// true — and it is gated by the unit tests below rather than by a swing.
///
/// **`val *= 100` and then `.Nf`, which is CPython's own order.** `formatter_unicode.c`'s `'%'`
/// branch rewrites the type to `'f'`, multiplies by 100 and appends the sign, so this is a plain
/// f64 multiply followed by [`fixed`] — not a rounding of `x` that is then scaled, which would
/// round twice. As with [`signed_fixed`], that makes `fixed.json` the gate for every digit here.
pub fn percent(x: f64, precision: usize) -> String {
    format!("{}%", fixed(x * 100.0, precision))
}

/// CPython's default precision for `%g`: six significant digits, from C. Nothing derives it —
/// every `:g` site in the engine writes a bare `{…:g}` and relies on this being what that means.
const G_PRECISION: usize = 6;

/// Python's `f"{x:g}"`, which Rust has no equivalent of at all.
///
/// `%g` is `%e` or `%f` chosen by the exponent, with trailing zeros stripped. The subtlety, and the
/// only part of this worth reading twice, is *which* exponent: it is the exponent of the value
/// **after** rounding to six significant digits, so `999999.5` prints as `1e+06` rather than as
/// `999999` — it ties to `1.00000e6` first, and only then is the form picked. Taking the mantissa
/// and exponent from `{:.5e}` gets that ordering right for free, where deciding the form from the
/// raw magnitude gets the boundary wrong in one direction and rounding twice gets it wrong in the
/// other.
///
/// The fixed branch reformats `x` rather than reusing the mantissa's digits, which looks like the
/// double rounding just ruled out and is not: C specifies `%g` as `%.{P-1-X}f` *of the original
/// value*, and both roundings here are correctly rounded from the exact binary value, so the
/// second one cannot move a digit the first one placed.
///
/// One of the eighteen `:g` sites is not prose at all — `benchmarks/flight_model.py:205` formats an
/// altitude into a **dict key** — so being wrong here is a lookup miss there rather than a wrong
/// sentence. Same function either way.
///
/// [`general`] at six, and gated by `format/general` rather than by `general_precision`, which
/// tables every other precision so the default is not tabled twice.
pub fn g(x: f64) -> String {
    general(x, G_PRECISION)
}

/// Python's `f"{x:.{precision}g}"` — [`g`] at a precision other than six. [M36 P4]
///
/// One caller so far, and it is prose: `analysis/dispersion.py`'s drift caveat prints a session's
/// pooled and within-session spreads at `:.3g`. The rule is `%g`'s with `P` moved, so both
/// thresholds move with it: exponent form when the rounded exponent is below -4 **or at least
/// `P`**, which at three puts the boundary at 1000 rather than at a million. `999.5` is `1e+03`
/// here where `:g` writes `999.5`, and a spread in rpm is a magnitude the caveat really prints.
///
/// CPython treats a precision of 0 as 1 (`format(1234.0, ".0g")` is `1e+03`), and so does this,
/// rather than asking `{:.*e}` for a precision of minus one.
pub fn general(x: f64, precision: usize) -> String {
    if x.is_nan() {
        return "nan".to_string();
    }
    if x.is_infinite() {
        return if x.is_sign_negative() { "-inf" } else { "inf" }.to_string();
    }
    let precision = precision.max(1);
    let scientific = format!("{:.*e}", precision - 1, x);
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("LowerExp always writes an `e`");
    let exponent: i32 = exponent
        .parse()
        .expect("LowerExp writes a decimal exponent");
    // C's rule is written `X < -4 || X >= P`; this is that condition as the half-open window it
    // describes, which is what clippy asks for and reads no worse. A precision too large for an
    // `i32` has no exponent it can reach, so saturating it keeps the window honest.
    let top = i32::try_from(precision).unwrap_or(i32::MAX);
    if !(-4..top).contains(&exponent) {
        // Python pads the exponent to two digits and always signs it, where Rust writes neither.
        let sign = if exponent < 0 { '-' } else { '+' };
        format!(
            "{}e{}{:02}",
            strip_trailing_zeros(mantissa),
            sign,
            exponent.abs()
        )
    } else {
        // Inside the window `exponent < precision`, so this cannot underflow.
        let decimals = (top - 1 - exponent) as usize;
        strip_trailing_zeros(&format!("{x:.decimals$}"))
    }
}

/// `%g`'s trailing-zero rule: drop them, then drop a bare trailing point.
///
/// Guarded on containing a `.` because `100000` must survive intact — stripping zeros from an
/// integer rendering would turn it into `1`, which is the kind of bug that only shows up on round
/// numbers and therefore on the values most likely to be in a screenshot.
fn strip_trailing_zeros(text: &str) -> String {
    if !text.contains('.') {
        return text.to_string();
    }
    let trimmed = text.trim_end_matches('0');
    trimmed.strip_suffix('.').unwrap_or(trimmed).to_string()
}

/// The exponent window CPython's `repr` stays out of: exponent form when `decpt <= -4 || decpt >
/// 16`, where `decpt` is the decimal point's position in the shortest round-trip digit string.
/// From `format_float_short`'s `'r'` branch, and nothing here derives it.
const REPR_MAX_DECPT: i32 = 16;
const REPR_MIN_DECPT: i32 = -4;

/// Python's `str(x)` for a float — the **fifth** portability edge, found by M22 P5.
///
/// ADR-032 §3 names three and P4 found a fourth; this is the one an f-string reaches with no
/// format spec at all. `f"aim under {band.high}"` is three of `mechanics.py`'s sentences, and in
/// Python 3 `str`, `repr` and `format(v, "")` are one function for a float, so all three spellings
/// land here.
///
/// **Rust's `{}` is the same shortest-round-trip digits and a different presentation**, which is
/// what makes this edge quiet: on every band value shipping today the two agree exactly, so
/// nothing currently fails. They part on three things, each of which a future band row could walk
/// into:
///
/// 1. An integral value. Python writes `4.0` (`Py_DTSF_ADD_DOT_0`), Rust writes `4`.
/// 2. The exponent window. Python writes `1e+16` and `1e-05`; Rust's `{}` never reaches for an
///    exponent and writes `10000000000000000` and `0.00001`. This is the same disagreement
///    `crates/contracts`' doc records about `serde_json`, arriving in prose instead of in JSON.
/// 3. `NaN`, which Rust capitalises and Python does not — [`fixed`] rewrites the same case.
///
/// Built from `{:e}`, for the reason [`g`] is: `LowerExp` is the shortest round-trip form, so the
/// digits are already the ones CPython's `_Py_dg_dtoa` would produce in mode 0 and this function
/// only has to decide where the point goes. Reformatting with `{}` and patching the result would
/// be deciding that from a rendering that has already thrown the exponent away.
pub fn repr(x: f64) -> String {
    if x.is_nan() {
        return "nan".to_string();
    }
    if x.is_infinite() {
        return if x.is_sign_negative() { "-inf" } else { "inf" }.to_string();
    }

    let scientific = format!("{x:e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("LowerExp always writes an `e`");
    let exponent: i32 = exponent
        .parse()
        .expect("LowerExp writes a decimal exponent");

    // `-0.0` reaches here as mantissa `-0`, and its sign is the whole point: Python writes `-0.0`
    // and a port that normalizes it away is wrong in prose the same way it would be wrong in JSON.
    let (sign, mantissa) = match mantissa.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", mantissa),
    };
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    // CPython's `decpt`: the value is `0.<digits> * 10**decpt`, where `{:e}` writes
    // `<d0>.<rest>e<exponent>` — one digit before the point, so the point sits one place further
    // right than the exponent.
    let decpt = exponent + 1;

    if decpt <= REPR_MIN_DECPT || decpt > REPR_MAX_DECPT {
        let head = &digits[..1];
        let tail = &digits[1..];
        let point = if tail.is_empty() {
            String::new()
        } else {
            format!(".{tail}")
        };
        // Python pads the exponent to two digits and always signs it, exactly as in `%g`.
        let power = decpt - 1;
        let power_sign = if power < 0 { '-' } else { '+' };
        return format!("{sign}{head}{point}e{power_sign}{:02}", power.abs());
    }

    if decpt <= 0 {
        // `0.00<digits>` — `-decpt` zeros between the point and the first significant digit.
        return format!("{sign}0.{}{digits}", "0".repeat(-decpt as usize));
    }
    let decpt = decpt as usize;
    if decpt >= digits.len() {
        // Every digit is left of the point, so the fraction is the forced `.0` and the padding in
        // between. `1e15` is the case that needs both: `1000000000000000.0`.
        let padding = "0".repeat(decpt - digits.len());
        return format!("{sign}{digits}{padding}.0");
    }
    format!("{sign}{}.{}", &digits[..decpt], &digits[decpt..])
}

/// `math.hypot(*coordinates)` — CPython's, not [`f64::hypot`] chained. [M22 P8]
///
/// **A fourth kind of edge, and the first that is not a string.** ADR-032 §3's three all reach the
/// sentences §3 compares exactly; this one reaches a **bool**. `analysis/flight.py` computes the
/// ball's speed as `math.hypot(vx, vy, vz)` and there is no three-argument `hypot` in Rust's std, so
/// a port reaches for `a.hypot(b).hypot(c)` or `(x²+y²+z²).sqrt()`. Both sit within 1 ulp of CPython
/// over the 919 points of a committed flight — and 1 ulp is the whole difference, because
/// `spin_solve::carry_window` constructs `high_plateau_min_rpm` so that the launch spin ratio lands
/// **exactly** on the coefficient table's last row. On the reference shot CPython's answer is
/// `40.546527999999995` where both approximations give `40.546528`, which is the launch speed to the
/// bit; the first makes `AeroCoefficients::clamped` true at that shoulder and the second makes it
/// false. A flipped clamp is a different flight, and `clamped` is compared exactly.
///
/// So this is CPython's `vector_norm` (`Modules/mathmodule.c`), transcribed: scale the largest
/// component into `[0.5, 1)` by a power of two, split each scaled component into two exactly
/// representable halves with Dekker's `T27` trick, accumulate the squares in three Neumaier
/// compensation registers, then correct the resulting `sqrt` by one Newton step taken in the same
/// compensated arithmetic. The result is correctly rounded, which neither approximation is.
///
/// **Only the three-argument call site uses it**, and that is deliberate rather than tidy.
/// `measure.rs` and `pivot.rs` already stand `f64::hypot` in for CPython's two-argument `math.hypot`,
/// which P5b recorded as a provable equivalence; this finding weakens "provable" to "within a ulp",
/// but every one of those answers reaches a float compared within `RTOL = 1e-9` and none reaches a
/// bool. Switching them would move numbers four committed stages already agree on, for no gate. The
/// table below covers the two-argument form anyway, so the day one of those sites needs it the
/// function is here and is already right.
///
/// Takes a slice because `math.hypot` is variadic. Non-finite inputs follow CPython: an infinite
/// component wins over everything including a NaN, and a NaN otherwise propagates.
pub fn hypot(coordinates: &[f64]) -> f64 {
    let mut vec: Vec<f64> = Vec::with_capacity(coordinates.len());
    let mut max = 0.0f64;
    let mut found_nan = false;
    for &coordinate in coordinates {
        let x = coordinate.abs();
        found_nan |= x.is_nan();
        if x > max {
            max = x;
        }
        vec.push(x);
    }
    vector_norm(&mut vec, max, found_nan)
}

/// Dekker's splitting constant, `ldexp(1.0, 27) + 1.0`. CPython's `T27`.
const T27: f64 = 134_217_729.0;

/// CPython's `vector_norm`, on absolute values with their maximum already found.
fn vector_norm(vec: &mut [f64], max: f64, found_nan: bool) -> f64 {
    if max.is_infinite() {
        return max;
    }
    if found_nan {
        return f64::NAN;
    }
    if max == 0.0 || vec.len() <= 1 {
        return max;
    }
    let max_e = frexp_exponent(max);
    if max_e < -1023 {
        // The squares would all underflow. CPython scales the whole vector up by `DBL_MIN` and
        // recurses, which is exact because `DBL_MIN` is a power of two.
        for x in vec.iter_mut() {
            *x /= f64::MIN_POSITIVE;
        }
        let scaled_max = max / f64::MIN_POSITIVE;
        return f64::MIN_POSITIVE * vector_norm(vec, scaled_max, found_nan);
    }
    let scale = power_of_two(-max_e);

    let mut csum = 1.0f64;
    let (mut frac1, mut frac2, mut frac3) = (0.0f64, 0.0f64, 0.0f64);
    for &coordinate in vec.iter() {
        // Lossless: `scale` is a power of two.
        let x = coordinate * scale;
        let t = x * T27;
        let hi = t - (t - x);
        let lo = x - hi;

        // `hi * hi`, `2*hi*lo` and `lo*lo` are each exact, so the only error left is in the three
        // sums — which is what the compensation registers carry.
        let term = hi * hi;
        let old = csum;
        csum += term;
        frac1 += (old - csum) + term;

        let term = 2.0 * hi * lo;
        let old = csum;
        csum += term;
        frac2 += (old - csum) + term;

        frac3 += lo * lo;
    }
    let h = (csum - 1.0 + (frac1 + frac2 + frac3)).sqrt();

    // One Newton step on `h² = sum`, taken in the same compensated arithmetic: subtract `h²` back
    // out of the running sum and divide the residue by `2h`. This is what makes the answer correctly
    // rounded rather than merely close.
    let t = h * T27;
    let hi = t - (t - h);
    let lo = h - hi;

    let term = -hi * hi;
    let old = csum;
    csum += term;
    frac1 += (old - csum) + term;

    let term = -2.0 * hi * lo;
    let old = csum;
    csum += term;
    frac2 += (old - csum) + term;

    let term = -lo * lo;
    let old = csum;
    csum += term;
    frac3 += (old - csum) + term;

    let x = csum - 1.0 + (frac1 + frac2 + frac3);
    (h + x / (2.0 * h)) / scale
}

/// C's `frexp` exponent: the `e` with `x = m * 2**e` and `0.5 <= |m| < 1`.
///
/// Read off the bits rather than through `log2`, which is neither exact nor monotone at the
/// boundaries. Subnormals carry no exponent of their own, so they are scaled into the normal range
/// first — by a power of two, so the scaling is exact.
fn frexp_exponent(x: f64) -> i32 {
    debug_assert!(x.is_finite() && x != 0.0);
    let field = ((x.to_bits() >> 52) & 0x7ff) as i32;
    if field == 0 {
        return frexp_exponent(x * power_of_two(64)) - 64;
    }
    field - 1022
}

/// `ldexp(1.0, e)` — `2**e`, exactly.
///
/// Built from the exponent field where that is available, and by `powi` below it. `powi` on a power
/// of two is exact until the result itself becomes subnormal, which is the only place it loses a bit
/// and is the same place C's `ldexp` does.
fn power_of_two(e: i32) -> f64 {
    if (-1022..=1023).contains(&e) {
        f64::from_bits(((e + 1023) as u64) << 52)
    } else {
        2.0f64.powi(e)
    }
}

/// `order.get(name, len(order))` — the rank `analysis/engine.py` sorts `unscored` by.
///
/// The fallback is the registry's length, so **every** unregistered name gets the same rank and a
/// stable sort therefore leaves them in the order they arrived in. That is the load-bearing half:
/// a port that sorts unknown names by name instead produces a different `unscored` order, and
/// `docs/CONFORMANCE.md` §3 compares list order exactly.
pub fn registry_rank<S: AsRef<str>>(name: &str, registry: &[S]) -> usize {
    registry
        .iter()
        .position(|entry| entry.as_ref() == name)
        .unwrap_or(registry.len())
}

/// A `dict` with Python's insertion order, because here the order is an answer.
///
/// ADR-032 §3's third edge, and the one that would have been hardest to diagnose: a Rust `HashMap`
/// destroys the property `analysis/engine.py` reads. `benchmarks/joint.py` and
/// `benchmarks/trajectory.py` each build a share-per-name dict in a fixed order, sort it by
/// descending share, and `engine.py` takes `next(iter(...))` off the front and interpolates the
/// **name** into `Measurement.detail` as "Largest contributor". On a tie the winner is whichever
/// came first in the original dict — so the iteration order of the model's metric list is part of
/// the answer, and a swing where two contributions tie gets a correct number under the wrong name.
///
/// A `Vec` of pairs rather than a third-party ordered map: the operations needed are insert, look
/// up, iterate and sort, the maps here hold three to six entries, and a dependency whose whole
/// value is `O(1)` lookup at n=6 would be paying in the currency ADR-032 §6 is careful about.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrderedMap<V> {
    entries: Vec<(String, V)>,
}

impl<V> OrderedMap<V> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Insert or overwrite, keeping an existing key in the position it was first inserted at —
    /// which is what `d[k] = v` does in Python and the reason this is not `Vec::push`.
    pub fn insert(&mut self, key: impl Into<String>, value: V) {
        let key = key.into();
        match self.entries.iter_mut().find(|(name, _)| *name == key) {
            Some(slot) => slot.1 = value,
            None => self.entries.push((key, value)),
        }
    }

    pub fn get(&self, key: &str) -> Option<&V> {
        self.entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.entries
            .iter()
            .map(|(name, value)| (name.as_str(), value))
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(name, _)| name.as_str())
    }

    /// `next(iter(d), default)` — what `engine.py` actually reads off a sorted share map.
    pub fn first_key(&self) -> Option<&str> {
        self.entries.first().map(|(name, _)| name.as_str())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// `dict(sorted(d.items(), key=…))` — a **stable** sort by an `f64` key, descending nothing.
    ///
    /// The key function returns what Python's `key=` returns, negation included, so the two call
    /// sites port as written: `-abs(share)` in `joint.py` and `-share` in `trajectory.py`. Those
    /// are not interchangeable — on `[-0.4, 0.4]` the first ties and the second does not — so
    /// neither is hard-coded here.
    ///
    /// `slice::sort_by` is stable, which is the whole requirement. **The vector table cannot prove
    /// that, and it is worth knowing which assurances come from where:** swapping this for
    /// `sort_unstable_by` passes all 10 ordering cases, because Rust's unstable sort is an
    /// insertion sort below 20 elements and these maps hold three to six. So the guarantee at the
    /// sizes the engine really uses comes from the documented stability of this call, and the table
    /// covers the mistakes it *can* see — an ordered map replaced by a sorted one, or a rank
    /// function that does not collide every unknown name. Both were confirmed by mutation; this one
    /// was confirmed to survive.
    ///
    /// A NaN key is the one input where the two languages are not defined to agree, and neither
    /// real caller can produce one: both guard their divisor above zero before dividing.
    pub fn sorted_by(&self, key: impl Fn(&str, &V) -> f64) -> Self
    where
        V: Clone,
    {
        let mut entries = self.entries.clone();
        entries.sort_by(|a, b| {
            key(&a.0, &a.1)
                .partial_cmp(&key(&b.0, &b.1))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Self { entries }
    }
}

impl<V> FromIterator<(String, V)> for OrderedMap<V> {
    fn from_iter<I: IntoIterator<Item = (String, V)>>(iter: I) -> Self {
        // Through `insert` rather than `collect` into the Vec, so a repeated key behaves the way
        // a dict comprehension over repeated keys does rather than producing two rows.
        let mut out = Self::new();
        for (key, value) in iter {
            out.insert(key, value);
        }
        out
    }
}

/// Python's `repr(s)` for a `str` — what every `{text!r}` in a parser warning writes. [M34 P2]
///
/// Warnings are compared exactly, and three of the parser's carry a `!r`: a cell's text, a missing
/// label, and the screen's title. Rust's `{:?}` is the call a port reaches for, and over the
/// `str_repr` table as recorded it gave a different string on 272 of 274 cases — agreeing only on
/// the two where CPython itself chose double quotes — for four separate reasons:
///
/// 1. **The quote.** `{:?}` always writes `"`. CPython writes `'` unless the text holds a `'` and
///    no `"`, and then writes `"` — so `it's` is `"it's"` and `say "hi"` is `'say "hi"'`.
/// 2. **Which quote is escaped.** Only the one CPython chose. `{:?}` escapes every `"` and no `'`.
/// 3. **The escape spelling.** CPython writes `\x00`, `\x1b`, `\xad`, `​` and `\U000e0001`,
///    with lowercase hex padded to two, four or eight digits by the code point's width. Rust writes
///    `\0` and `\u{1b}`.
/// 4. **A combining mark.** Rust's `Debug` escapes a grapheme extender wherever it stands, so `é`
///    spelled `e` + U+0301 comes out as `e\u{301}`; CPython keeps it, because Mn is printable.
///
/// Everything else in CPython's `unicode_repr` is ported as written: `\\`, `\t`, `\n` and `\r` by
/// name, other C0 controls and DEL as `\xhh`, printable ASCII as itself, and non-ASCII kept or
/// escaped by [`is_printable`].
pub fn str_repr(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(text.len() + 2);
    out.push(quote);
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c < ' ' || c == '\x7f' => out.push_str(&format!("\\x{:02x}", c as u32)),
            c if c.is_ascii() || is_printable(c) => out.push(c),
            c => {
                let code = c as u32;
                let escaped = if code <= 0xff {
                    format!("\\x{code:02x}")
                } else if code <= 0xffff {
                    format!("\\u{code:04x}")
                } else {
                    format!("\\U{code:08x}")
                };
                out.push_str(&escaped);
            }
        }
    }
    out.push(quote);
    out
}

/// CPython's `Py_UNICODE_ISPRINTABLE` for a non-ASCII character: false for the categories `Cc`,
/// `Cf`, `Cs`, `Co`, `Cn`, `Zl`, `Zp` and `Zs`.
///
/// **Read off `core`'s own table rather than carried as one here**, because that table is built from
/// exactly that rule (`library/core/src/unicode/printable.py`) and is not public. `str::escape_debug`
/// is the door to it: past the first character it escapes a character only when that table says it
/// is not printable, and — unlike `char::escape_debug` and `{:?}` — leaves a grapheme extender
/// alone. So a probe of `a` followed by the character comes back unchanged exactly when CPython
/// would print the character as itself.
///
/// The one place the two can differ is the Unicode version under each table — 16.0 in Rust 1.87,
/// 15.1 in CPython 3.13 — and measured over every code point that is all they differ by: they agree
/// on everything 15.1 had assigned, and part on exactly the 5,185 characters 16.0 added, which Rust
/// prints and CPython escapes as unassigned. None is a character a launch monitor prints, so the
/// table carries none, and the gap is left standing rather than closed with a copy of CPython's
/// table. A toolchain on a later Unicode widens it the same way, and only there.
fn is_printable(c: char) -> bool {
    let mut probe = String::with_capacity(8);
    probe.push('a');
    probe.push(c);
    probe.escape_debug().skip(1).eq(std::iter::once(c))
}

/// Python's `a // b` on two floats — CPython's `float_floor_div`, not `(a / b).floor()`. [M34 P2]
///
/// `_Cell.text` sorts a tile's value boxes into lines by `int(center_y // bucket)`, where the bucket
/// is the label's height times 0.6. CPython computes `//` from `fmod`: the quotient is
/// `(a - fmod(a, b)) / b`, corrected by one when the remainder's sign disagrees with `b`'s, then
/// floored and nudged up if the floor fell more than a half below it. That is exact where `a / b`
/// is not, and the two part company on precisely the parser's geometry: a center sitting on a
/// multiple of a bucket the binary point cannot represent. `21.0 // 4.2` is `4.0` — the remainder
/// is `4.199999999999999` — while `21.0 / 4.2` rounds to exactly `5.0`, which is a 7 px label and a
/// value box centred at y=21, put on a different line. Over the integer-pixel grid PaddleOCR
/// returns (centers on the half-pixel to 2,000, labels 1-80 px tall) that is 300 of 320,080 pairs,
/// and over the same span drawn uniformly at random it is none of 200,000 — so the `floor_div`
/// table carries all 300, and `(a / b).floor()` fails every one of them, as an integer too.
///
/// A zero divisor panics, as CPython raises `ZeroDivisionError`; the parser's bucket is floored at
/// `1e-6` first and never reaches it. A zero quotient keeps the sign of `a / b`, so `-0.0 // 0.6`
/// is `-0.0`, which `int()` then forgets.
pub fn floor_div(a: f64, b: f64) -> f64 {
    assert!(b != 0.0, "float floor division by zero");
    let remainder = a % b;
    let mut div = (a - remainder) / b;
    // `if (mod)` in C is true for a NaN too, which `!= 0.0` keeps.
    if remainder != 0.0 && ((b < 0.0) != (remainder < 0.0)) {
        div -= 1.0;
    }
    if div != 0.0 {
        let floored = div.floor();
        if div - floored > 0.5 {
            floored + 1.0
        } else {
            floored
        }
    } else {
        0.0f64.copysign(a / b)
    }
}

/// Python's whitespace: `str.isspace`, `str.split()`, `str.strip()` and `re`'s `\s` all use this
/// one set (`Py_UNICODE_ISSPACE`). [M34 P2]
///
/// It is Rust's [`char::is_whitespace`] plus **the four C0 separators U+001C–U+001F**, which Python
/// counts as whitespace (their bidirectional class is `B` or `S`) and Unicode's `White_Space`
/// property does not. Measured over every code point against the `text_case` table's two whole-set
/// cases: those four are the only difference, in either direction. A port on `split_whitespace`
/// therefore reads a cell holding `12\x1c3` as one token where Python reads two.
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\x1c'..='\x1f')
}

/// Python's `str.upper()`. [M34 P2]
///
/// Rust's [`str::to_uppercase`] is the same full mapping — `ß` to `SS`, `ﬁ` to `FI`, `ŉ` to `ʼN`,
/// three characters out of `ΐ` — because both apply `SpecialCasing.txt`'s unconditional rules on
/// top of `UnicodeData.txt`, and neither has a context-sensitive rule for upper case. What differs
/// is only the Unicode version underneath (16.0 against 15.1, as in [`is_printable`]), and measured
/// over every code point it is 27 of them: 25 that 16.0 assigned, Garay's letters among them, and
/// two older ones it gave a new capital, `ƛ` (U+019B) and `ɤ` (U+0264). Rust maps all 27 and
/// CPython 3.13 none, and none is on a launch monitor's screen. A function of its own all the same,
/// so the parser names one CPython operation in one place, and the day the two drift the fix has an
/// address.
pub fn upper(text: &str) -> String {
    text.to_uppercase()
}

/// Python's `str.lower()`. [M36 P4]
///
/// The many-shot layer's case fold: `golfer.py::slugify` lowers a display name before it becomes
/// an id, the golfer store sorts by `display_name.lower()`, and `club.py` and `club_spec.py` lower a
/// club name before matching an alias. Rust's [`str::to_lowercase`] is the same full mapping —
/// `İ` to `i` and a combining dot above, the Kelvin sign to an ASCII `k`, the ohm sign to `ω` —
/// and, unlike [`upper`], it is the one case operation that reads a character's **neighbours**:
/// both languages apply Unicode's `Final_Sigma` context, so `ΑΣ` lowers to `ας` and `ΑΣΑ` to
/// `ασα`, skipping case-ignorable characters (an apostrophe, a combining mark, a soft hyphen) in
/// both directions by the same rule (`handle_capital_sigma` in CPython, `map_uppercase_sigma` in
/// `core`).
///
/// What differs is again only the Unicode version underneath, 16.0 against CPython 3.13's 15.1.
/// `format/lower` records every code point CPython lowers to something else — 1,433 of them — and
/// Rust agrees on every one, `Final_Sigma`'s string cases included. Measured the other way over
/// every code point, Rust lowers 27 that CPython 3.13 leaves alone, each a capital 16.0 added:
/// Cyrillic capital TJE (U+1C89), four Latin capitals in U+A7CB–U+A7DC (the new partners of `ɤ` and `ƛ`
/// among them), and Garay's 22 (U+10D50–U+10D65). None is on a golfer's keyboard. As with
/// `upper`, a function of its own so the operation has one address the day the two drift.
pub fn lower(text: &str) -> String {
    text.to_lowercase()
}

/// Python's `str.split()` with no argument: split on runs of [`is_space`], dropping empty pieces,
/// so leading and trailing whitespace produce nothing. [M34 P2]
///
/// `split_whitespace` is the same algorithm on [`char::is_whitespace`], so it differs exactly where
/// that does — on U+001C–U+001F.
pub fn split(text: &str) -> impl Iterator<Item = &str> {
    text.split(is_space).filter(|piece| !piece.is_empty())
}

/// Python's `str.strip()` with no argument. [M34 P2]
///
/// [`str::trim`] differs on U+001C–U+001F, as [`split`] does. `_Cell.text` strips every box before
/// joining a tile's text, so a box of only a separator is dropped in Python and kept by `trim`.
pub fn strip(text: &str) -> &str {
    text.trim_matches(is_space)
}

/// Python's `re.sub(r"\s+", "", text)` — every whitespace character deleted. [M34 P2]
///
/// `_is_blank`'s comparison form. `\s` in a `str` pattern is [`is_space`], measured whole in the
/// `text_case` table, so this needs no regex: deleting every run of a class is deleting every member.
pub fn strip_space(text: &str) -> String {
    text.chars().filter(|&c| !is_space(c)).collect()
}

/// Python's `sum(values)` over floats — **compensated** since CPython 3.12. [M34 P2]
///
/// CPython 3.12 replaced the left-to-right float sum with Neumaier's improvement of Kahan's: the
/// running total keeps a separate compensation term, fed by whichever addend lost low bits, and
/// adds it back once at the end. So `sum([0.1] * 10)` is `1.0` where `values.iter().sum()` gives
/// `0.9999999999999999`, and over the `sum` table as recorded the two differed on 82 of 149 lists:
/// 80 by the compensation, and the two lists of negative zeros by the start value below.
///
/// Transcribed from `builtin_sum_impl`, details included:
///
/// - The start is the `int` 0, which the first float absorbs as `0 + x`: so the running total
///   begins at `0.0 + first` and the loop runs over the rest. `0 + -0.0` is `0.0`, which is why a
///   list of negative zeros sums to `0.0` here, where Rust's `Sum` (which starts from `-0.0`)
///   answers `-0.0`.
/// - The compensation is added back only if it is non-zero and finite, so an overflowed or
///   infinite total stays infinite instead of becoming `inf + nan`.
/// - An empty list is CPython's `int` 0; this returns `0.0`, the float the parser's
///   `sum(ocr) / len(ocr) if ocr else 0.0` would never divide anyway.
///
/// Where this bites is narrower than it looks. A real OCR confidence is a float32 widened to a
/// float64, and fifty 24-bit values sum *exactly* in 53 bits, so compensation never moves a photo's
/// mean; the synthetic screens' constant `0.95` is what it moves (`format/sum` carries both).
pub fn sum(values: &[f64]) -> f64 {
    let Some((&first, rest)) = values.split_first() else {
        return 0.0;
    };
    let mut total = 0.0 + first;
    let mut compensation = 0.0f64;
    for &x in rest {
        let t = total + x;
        if total.abs() >= x.abs() {
            compensation += (total - t) + x;
        } else {
            compensation += (x - t) + total;
        }
        total = t;
    }
    if compensation != 0.0 && compensation.is_finite() {
        total += compensation;
    }
    total
}

#[cfg(test)]
mod tests {

    /// CPython's own answers for [`hypot`], recorded from `math.hypot` at the interpreter this repo
    /// runs (3.13), and compared with **no tolerance** — the same rule `spec/vectors/format/` is read
    /// under, for the same reason: a norm is either the same norm or it is a different one.
    ///
    /// Not a vector family of its own, because thirty-odd cases do not earn 266 KB on disk and the
    /// interesting one is a single value. That value is the first row: the reference shot's launch
    /// velocity, where CPython returns `40.546527999999995` and both of the approximations a port
    /// reaches for return `40.546528`. It is the case that flipped an `AeroCoefficients::clamped`.
    ///
    /// The decimal literals round-trip exactly: Python's `repr` is the shortest string that recovers
    /// the f64, and Rust's parser is correctly rounded, so both sides read the same bits.
    // Every literal here is CPython's `repr` and is the answer being compared, so clippy's
    // truncation advice would delete the test's whole subject.
    #[allow(clippy::excessive_precision)]
    #[test]
    fn hypot_is_cpythons_norm_to_the_bit() {
        let cases: &[(&[f64], f64)] = &[
            // the reference shot's launch velocity - the case that exposed the edge
            (
                &[37.71680479622381, 14.464487278077161, -3.498882193026264],
                40.546527999999995,
            ),
            // a planar launch: the third leg is exactly zero
            (&[37.878747875782906, 14.464487278077161, 0.0], 40.546528),
            // the two-argument call, on a Pythagorean triple
            (&[3.0, 4.0], 5.0),
            // a three-dimensional Pythagorean quadruple
            (&[3.0, 4.0, 12.0], 13.0),
            // three equal components
            (&[1.0, 1.0, 1.0], 1.7320508075688772),
            // the origin
            (&[0.0, 0.0, 0.0], 0.0),
            // one nonzero leg
            (&[0.0, 0.0, 7.5], 7.5),
            // near the top of the range, where a naive sum of squares overflows
            (&[1e+300, 1e+300, 1e+300], 1.7320508075688774e+300),
            // near the bottom, where a naive sum of squares underflows to zero
            (&[1e-300, 1e-300, 1e-300], 1.7320508075688774e-300),
            // subnormal components, which take `vector_norm`'s scale-up recursion
            (&[5e-324, 1e-323, 0.0], 1e-323),
            // one dominant leg and two negligible ones
            (&[1.0, 1e-20, 1e-20], 1.0),
            // signs are dropped before the norm
            (&[-8.5, 2.25, -0.125], 8.793641168480779),
            (
                &[-4.302126582058349, 12.604985867540321, 56.23762754236894],
                57.79329297974946,
            ),
            (
                &[-28.21782113518629, 43.32995349191371, 55.10545816059765],
                75.56680367938127,
            ),
            (
                &[8.688204126073032, 57.26128921867421, -43.092760556108885],
                72.18951548713359,
            ),
            (
                &[-9.650199372969936, -37.93333673644963, -47.81685334498535],
                61.79414088499641,
            ),
            (
                &[36.18086078221994, 16.197892640899568, 32.30256013977201],
                51.13591501608314,
            ),
            (
                &[-18.466797875224145, 22.159156891758542, -36.62483707071138],
                46.62005521633482,
            ),
            (
                &[32.2267325463023, 17.17514361458167, -10.56784588306325],
                38.01614414167463,
            ),
            (
                &[2.8844628603140094, 46.34656590484248, 20.010645644232582],
                50.56431781665291,
            ),
            (
                &[-44.14745269830289, -46.03582223032326, -54.224187518981395],
                83.71712501193812,
            ),
            (
                &[-22.071475291040542, -32.7537535530082, -35.930414548996225],
                53.3943169541191,
            ),
            (
                &[24.857240227320787, 10.092082510283234, -36.66011706224079],
                45.427928679726286,
            ),
            (
                &[-29.47097969535177, -45.327051172325746, 56.584188638744735],
                78.26142482786473,
            ),
            (
                &[-13.601094908174332, -6.402975062770643, 38.07802136788585],
                40.93804567451651,
            ),
            (
                &[-26.654454924917452, 57.575524998844784, 29.928967743348068],
                70.15086711095898,
            ),
            (
                &[0.6794748600911573, 0.35313397073359276],
                0.7657608548247761,
            ),
            (
                &[-0.9210141334523234, -0.5288222418100772],
                1.0620357797418927,
            ),
            (
                &[-0.7078027112490057, -0.9427642305225303],
                1.178893155635481,
            ),
            (
                &[0.22882542721280696, 0.26187964991099677],
                0.34776720256031884,
            ),
            (
                &[-0.31280800760391547, 0.10757630181556688],
                0.3307892234239882,
            ),
            (
                &[0.3444364422093704, -0.3547993136723333],
                0.4944886406220144,
            ),
        ];
        let mut differences = Vec::new();
        for (args, want) in cases {
            let got = hypot(args);
            if got.to_bits() != want.to_bits() {
                differences.push(format!("hypot({args:?}) = {got:?}, CPython says {want:?}"));
            }
        }
        assert!(
            differences.is_empty(),
            "{} of {} cases differ:\n{}",
            differences.len(),
            cases.len(),
            differences.join("\n")
        );
    }

    /// The first row of the table above, stated as the difference it makes rather than as a value:
    /// **neither approximation a port reaches for is CPython's answer**, and both differ by exactly
    /// one ulp. This is the assertion that fails if somebody simplifies [`hypot`] back to a chain.
    #[test]
    fn neither_chained_hypot_nor_a_naive_sqrt_is_cpythons_answer() {
        let v = [37.71680479622381, 14.464487278077161, -3.498882193026264];
        let cpython = hypot(&v);
        let chained = v[0].hypot(v[1]).hypot(v[2]);
        let naive = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        assert_eq!(cpython, 40.546527999999995);
        assert_eq!(chained, 40.546528);
        assert_eq!(naive, 40.546528);
        // One ulp apart, and on the side that matters: CPython's is the smaller.
        assert_eq!(chained.to_bits() - cpython.to_bits(), 1);
    }

    /// Non-finite inputs follow CPython's order of precedence: an infinity wins over a NaN, and a
    /// NaN otherwise propagates. Both are bugs wherever they arrive from, which is why the two
    /// languages should at least agree about which bug.
    #[test]
    fn an_infinity_beats_a_nan_and_a_nan_otherwise_propagates() {
        assert_eq!(hypot(&[3.0, f64::INFINITY, f64::NAN]), f64::INFINITY);
        assert_eq!(hypot(&[f64::NEG_INFINITY, 1.0]), f64::INFINITY);
        assert!(hypot(&[3.0, f64::NAN, 4.0]).is_nan());
        // One coordinate is itself, absolute value and all — `vector_norm` returns `max` for n <= 1.
        assert_eq!(hypot(&[-6.25]), 6.25);
        assert_eq!(hypot(&[]), 0.0);
    }
    /// The three places [`repr`] and Rust's `{}` part company, named rather than left to the
    /// 362-case table: a reader of this module should be able to see why the function exists
    /// without opening a vector.
    #[test]
    fn repr_differs_from_rusts_display_in_exactly_three_places() {
        // 1. An integral value keeps its `.0` — `Py_DTSF_ADD_DOT_0`.
        assert_eq!(repr(4.0), "4.0");
        assert_eq!(format!("{}", 4.0_f64), "4");
        // 2. The exponent window, which Rust's `{}` does not have at all.
        assert_eq!(repr(1e16), "1e+16");
        assert_eq!(repr(1e-5), "1e-05");
        assert_eq!(format!("{}", 1e16_f64), "10000000000000000");
        // 3. A NaN, which Rust capitalises.
        assert_eq!(repr(f64::NAN), "nan");
    }

    /// And the boundaries themselves, from both sides, because an off-by-one in `decpt` is the
    /// mistake this function invites.
    #[test]
    fn repr_switches_form_at_cpythons_thresholds_and_not_a_decade_early() {
        assert_eq!(repr(1e15), "1000000000000000.0");
        assert_eq!(repr(1e16), "1e+16");
        assert_eq!(repr(1e-4), "0.0001");
        assert_eq!(repr(1e-5), "1e-05");
    }

    /// A signed zero survives, for the reason `round(x, n)`'s does: CPython keeps the sign and a
    /// port that normalizes it prints a different number.
    #[test]
    fn repr_keeps_a_negative_zero() {
        assert_eq!(repr(-0.0), "-0.0");
        assert_eq!(repr(0.0), "0.0");
    }

    use super::*;

    // The cases here are the ones worth reading in place. Breadth is `tests/format.rs`'s job, and
    // duplicating it would mean two tables to keep true.

    #[test]
    fn one_arg_round_is_half_to_even() {
        assert_eq!(round_half_even(0.5), 0.0);
        assert_eq!(round_half_even(1.5), 2.0);
        assert_eq!(round_half_even(2.5), 2.0);
        assert_eq!(round_half_even(-2.5), -2.0);
        // Not a tie, however much it looks like one: the nearest f64 below the half.
        assert_eq!(round_half_even(0.49999999999999994), 0.0);
        assert_eq!(round_index(-0.5), 0);
    }

    #[test]
    fn round_to_beats_the_scaled_form() {
        // Each of these is a value the scaled port gets wrong, with what it would have said.
        assert_eq!(round_to(0.215, 2), 0.21); // scaled: 0.22, because 0.215 * 100.0 is exactly 21.5
        assert_eq!(round_to(5.565, 2), 5.57); // scaled: 5.56, the same error in the other direction
        assert_eq!(round_to(2.675, 2), 2.67); // scaled with ties-away: 2.68
        assert_eq!(round_to(0.125, 2), 0.12); // an exact tie, so to even where "half up" says 0.13
                                              // And two whose decimal look disagrees with their binary value in the first place.
        assert_eq!(round_to(0.145, 2), 0.14); // exactly 0.14499999999999999000799…
        assert_eq!(round_to(1.005, 2), 1.0);
    }

    #[test]
    fn round_to_passes_non_finite_through() {
        assert!(round_to(f64::NAN, 4).is_nan());
        assert_eq!(round_to(f64::INFINITY, 4), f64::INFINITY);
        assert_eq!(round_to(f64::NEG_INFINITY, 2), f64::NEG_INFINITY);
    }

    #[test]
    fn fixed_keeps_the_sign_on_a_negative_zero_and_lowercases_a_nan() {
        assert_eq!(fixed(-0.001, 2), "-0.00");
        assert_eq!(fixed(-0.0, 2), "-0.00");
        assert_eq!(fixed(f64::NAN, 2), "nan");
        assert_eq!(fixed(f64::INFINITY, 0), "inf");
        assert_eq!(fixed(0.5, 0), "0");
        assert_eq!(fixed(1.5, 0), "2");
    }

    #[test]
    fn g_picks_its_form_from_the_rounded_exponent() {
        // The boundary that a port deciding the form first gets wrong.
        assert_eq!(g(999999.5), "1e+06");
        assert_eq!(g(100000.0), "100000");
        assert_eq!(g(1000000.0), "1e+06");
        assert_eq!(g(0.0001), "0.0001");
        assert_eq!(g(0.00001), "1e-05");
    }

    #[test]
    fn g_strips_trailing_zeros_without_touching_an_integer() {
        assert_eq!(g(2.8), "2.8");
        assert_eq!(g(90.0), "90");
        assert_eq!(g(1500.0), "1500");
        assert_eq!(g(0.0), "0");
        assert_eq!(g(-0.0), "-0");
        assert_eq!(g(f64::NAN), "nan");
        assert_eq!(g(f64::NEG_INFINITY), "-inf");
    }

    #[test]
    fn g_ties_to_even_at_the_sixth_significant_digit() {
        assert_eq!(g(123456.5), "123456");
        assert_eq!(g(123457.5), "123458");
    }

    /// `:.3g` moves both thresholds with the precision, and that is the whole of the difference
    /// from `:g`: the exponent form starts at 1000. Each answer is CPython's `format(x, ".3g")`.
    #[test]
    fn general_moves_the_exponent_threshold_with_the_precision() {
        assert_eq!(general(999.5, 3), "1e+03");
        assert_eq!(g(999.5), "999.5");
        assert_eq!(general(1000.0, 3), "1e+03");
        assert_eq!(general(100.0, 3), "100");
        assert_eq!(general(9.996, 3), "10");
        assert_eq!(general(9.996e-05, 3), "0.0001");
        assert_eq!(general(12.25, 3), "12.2");
        assert_eq!(general(12.75, 3), "12.8");
        assert_eq!(general(-0.0, 3), "-0");
    }

    /// CPython reads a precision of 0 as 1, where `{:.*e}` would be asked for minus one. No caller
    /// formats at `.0g`, which is why the table carries none; the rule is pinned here instead, from
    /// CPython's answers.
    #[test]
    fn a_zero_precision_is_cpythons_one() {
        assert_eq!(general(1234.0, 0), "1e+03");
        assert_eq!(general(1234.0, 1), "1e+03");
        assert_eq!(general(2.5, 0), "2");
        assert_eq!(general(3.5, 0), "4");
        assert_eq!(general(0.000123456, 0), "0.0001");
    }

    #[test]
    fn registry_rank_collides_every_unknown_name() {
        let registry = ["tempo", "head_sway", "finish_balance"];
        assert_eq!(registry_rank("head_sway", &registry), 1);
        assert_eq!(registry_rank("zzz", &registry), 3);
        assert_eq!(registry_rank("aaa", &registry), 3);
    }

    #[test]
    fn ordered_map_keeps_insertion_order_through_a_tie() {
        let map: OrderedMap<f64> = ["b", "a", "c"]
            .iter()
            .map(|name| (name.to_string(), 0.5))
            .collect();
        let sorted = map.sorted_by(|_, value| -value.abs());
        assert_eq!(sorted.keys().collect::<Vec<_>>(), ["b", "a", "c"]);
        assert_eq!(sorted.first_key(), Some("b"));
    }

    #[test]
    fn ordered_map_overwrite_does_not_move_a_key() {
        let mut map = OrderedMap::new();
        map.insert("first", 1.0);
        map.insert("second", 2.0);
        map.insert("first", 9.0);
        assert_eq!(map.keys().collect::<Vec<_>>(), ["first", "second"]);
        assert_eq!(map.get("first"), Some(&9.0));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn the_two_real_key_functions_disagree_on_a_signed_pair() {
        let map: OrderedMap<f64> = [("low", -0.4), ("high", 0.4)]
            .iter()
            .map(|(name, value)| (name.to_string(), *value))
            .collect();
        // `-abs` ties them, so insertion order wins and `low` leads.
        assert_eq!(map.sorted_by(|_, v| -v.abs()).first_key(), Some("low"));
        // `-value` does not, so the positive share leads.
        assert_eq!(map.sorted_by(|_, v| -v).first_key(), Some("high"));
    }

    /// `:+.Nf` is [`fixed`] plus a sign, pinned against CPython on the three cases where that
    /// composition is not obvious. [M22 P6]
    ///
    /// `-0.0` is the one that matters: a sign test on the *value* writes `+-0.000` here, because
    /// `-0.0 < 0.0` is false. The digits themselves are `fixed`'s and gated by `fixed.json`, so
    /// this is a test of the sign rule and deliberately not a second rounding table.
    #[test]
    fn a_forced_sign_is_added_only_where_there_is_none() {
        for (value, precision, want) in [
            (0.0, 3, "+0.000"),
            (-0.0, 3, "-0.000"),
            (0.05, 3, "+0.050"),
            (-0.05, 3, "-0.050"),
            (1.0 / 60.0, 3, "+0.017"),
            (f64::NAN, 3, "+nan"),
            (f64::INFINITY, 3, "+inf"),
            (f64::NEG_INFINITY, 3, "-inf"),
        ] {
            assert_eq!(signed_fixed(value, precision), want, "{value:?}");
        }
    }

    /// `:.N%` multiplies by 100 and *then* rounds, which is CPython's order. [M22 P6]
    ///
    /// `0.015` is the case that tells the two orders apart at `.0%`: the product is `1.5`, which
    /// ties and goes to `2` under half-to-even, where rounding `0.015` first and scaling after
    /// would answer `1`. `-0.0` keeps its sign here too, for the same reason `fixed` does.
    #[test]
    fn a_percentage_scales_before_it_rounds() {
        for (value, precision, want) in [
            (0.611_111_111_111_111, 0, "61%"),
            (0.735_849_056_603_773_6, 0, "74%"),
            (0.0, 0, "0%"),
            (1.0, 0, "100%"),
            (0.005, 0, "0%"),
            (0.015, 0, "2%"),
            (0.125, 0, "12%"),
            (-0.0, 0, "-0%"),
            (f64::NAN, 0, "nan%"),
        ] {
            assert_eq!(percent(value, precision), want, "{value:?}");
        }
    }
}
