"""The ball-flight constants: loading, reading, and the extrapolation the corpus actually needs.

Like `test_joint.py` and `test_distributions.py`, these run against the **real** committed
`flight_model_v1.json` rather than a fixture. The artifact ships inside the wheel, so the failure
worth catching is someone re-sourcing it into a shape the loader cannot read — which a fixture
would hide.

They also run on a base install with no extras, which is what makes the file worth committing at
all: published constants and stdlib arithmetic, no fitting stage and nothing to install (ADR-022,
ADR-027 §Decision 2).
"""

from __future__ import annotations

import math

import pytest

from golf_coach.analysis.benchmarks import (
    coefficients_for,
    flight_dataset_info,
    load_flight_model,
)

#: The two 2026-08-10 reference shots — the only ones on disk carrying a full launch-condition set
#: *and* HD Golf's own carry beside it, and therefore M15 P4's validation set (ADR-027 §Context 2).
#: Ball speed in mph, spin in rpm.
_VALIDATION_SHOTS = {"2026-08-10-1": (90.7, 5991.0), "2026-08-10-2": (90.5, 8100.0)}


def test_dataset_provenance_is_present_and_says_nothing_was_fitted() -> None:
    info = flight_dataset_info()
    assert info.name == "flight_model_v1"
    assert info.fitted is False, (
        "the day something here is fitted, ADR-022's fitting-stage rules apply and this "
        "artifact stops being the clubs/catalogue.py shape"
    )
    assert "not a fit" in info.note.lower()


def test_every_block_carries_its_own_citation() -> None:
    """Five blocks, four sources — the reason provenance sits per block and not once at the top.

    M15 P6 made this five without making it five sources, and that is the interesting case rather
    than a weakening: `atmosphere` and `atmosphere_profile` are the sea-level row and the lowest
    layer of *one* standard, so they cite the same document at different clauses. Distinct
    citation text is still required of them — a block that could not say which part of ISO 2533 it
    is would be exactly the copied provenance this test exists to catch.
    """
    model = load_flight_model()
    blocks = [
        model.ball,
        model.atmosphere,
        model.atmosphere_profile,
        model.spin_decay,
        model.coefficients,
    ]
    for block in blocks:
        assert block.source.citation.strip()
        assert block.source.url.startswith("https://")
        assert block.source.note.strip()
    citations = {block.source.citation for block in blocks}
    assert len(citations) == len(blocks), "two blocks quoting one source is a copied provenance"

    documents = {block.source.url for block in blocks}
    assert len(documents) == 4, "five blocks, four documents — the two ISO 2533 blocks share one"
    assert model.atmosphere.source.url == model.atmosphere_profile.source.url


def test_the_ball_conforms_to_the_rules_it_cites() -> None:
    ball = load_flight_model().ball
    # R&A/USGA Equipment Rules Part 4: not heavier than 45.93 g, not smaller than 42.67 mm.
    assert ball.mass_kg <= 0.04593
    assert ball.diameter_m >= 0.04267
    assert ball.radius_m == ball.diameter_m / 2
    assert math.isclose(ball.frontal_area_m2, math.pi * ball.radius_m**2)


def test_the_rows_ascend_and_each_row_averages_its_two_orientations() -> None:
    rows = load_flight_model().coefficients.rows
    ratios = [row.spin_ratio for row in rows]
    assert ratios == sorted(ratios)
    assert len(set(ratios)) == len(ratios)
    for row in rows:
        assert math.isclose(row.cl, (row.cl_pp + row.cl_ph) / 2)
        assert math.isclose(row.cd, (row.cd_pp + row.cd_ph) / 2)


def test_lift_rises_with_spin_ratio_across_the_measured_range() -> None:
    """A physical pin, and the one that catches a table entered upside down.

    A golf ball's lift coefficient increases with spin ratio over the whole range anyone has
    published. Nothing downstream would notice a reversed table — it would simply fly every shot
    wrong — so the monotonicity is asserted here rather than assumed.
    """
    rows = load_flight_model().coefficients.rows
    lifts = [row.cl for row in rows]
    assert lifts == sorted(lifts)
    assert lifts[0] < lifts[-1]


def test_reading_a_measured_row_returns_that_row_unclamped() -> None:
    table = load_flight_model().coefficients
    middle = table.rows[len(table.rows) // 2]
    read = table.coefficients_for(middle.spin_ratio)
    assert math.isclose(read.cl, middle.cl)
    assert math.isclose(read.cd, middle.cd)
    assert read.clamped is False


def test_reading_between_rows_interpolates_and_stays_between_them() -> None:
    table = load_flight_model().coefficients
    low, high = table.rows[0], table.rows[1]
    midpoint = (low.spin_ratio + high.spin_ratio) / 2
    read = table.coefficients_for(midpoint)
    assert read.clamped is False
    assert math.isclose(read.cl, (low.cl + high.cl) / 2)
    assert min(low.cd, high.cd) <= read.cd <= max(low.cd, high.cd)


def test_outside_the_measured_range_the_end_rows_are_held_and_the_read_says_so() -> None:
    """Held, not extended — Smits and Smith measured lift saturating past this table's ceiling.

    The `clamped` flag is the whole point of the return type: without it a caller cannot tell a
    measurement from an extrapolation, and on every shot stored so far the answer is
    extrapolation.
    """
    table = load_flight_model().coefficients
    first, last = table.rows[0], table.rows[-1]

    below = table.coefficients_for(table.spin_ratio_min - 0.05)
    assert (below.cl, below.cd, below.clamped) == (first.cl, first.cd, True)

    above = table.coefficients_for(table.spin_ratio_max * 3)
    assert (above.cl, above.cd, above.clamped) == (last.cl, last.cd, True)


def test_both_validation_shots_launch_above_the_published_table() -> None:
    """The measurement behind M15 P2's central caveat, pinned so it cannot rot silently.

    The stored rows are eight points along a *driver's* flight and stop at a spin ratio of 0.284.
    A 7 iron launches well past that, and the spin ratio only climbs from there as the ball slows
    faster than its spin decays — so both reference shots are flown entirely on the clamp. If the
    table is ever extended to cover iron spin ratios, this test fails and the docstrings and
    ADR-027's addendum that say otherwise have to be rewritten in the same commit.
    """
    model = load_flight_model()
    ceiling = model.coefficients.spin_ratio_max
    for shot, (ball_speed_mph, spin_rpm) in _VALIDATION_SHOTS.items():
        speed_m_s = ball_speed_mph * 0.44704
        spin_rad_s = spin_rpm * 2 * math.pi / 60
        ratio = model.spin_ratio(speed_m_s, spin_rad_s)
        assert ratio is not None
        assert ratio > ceiling, f"{shot} was expected to launch above the table, at S={ratio:.3f}"
        assert model.coefficients_at(speed_m_s, spin_rad_s).clamped is True


def test_a_ball_at_a_standstill_has_no_spin_ratio_and_no_coefficients() -> None:
    """`None` rather than infinity, so a division by zero is refused where it happens."""
    model = load_flight_model()
    assert model.spin_ratio(0.0, 300.0) is None
    assert model.coefficients_at(0.0, 300.0) is None
    assert model.spin_ratio(-1.0, 300.0) is None


def test_spin_decays_exponentially_at_the_published_rate() -> None:
    decay = load_flight_model().spin_decay
    assert decay.kind == "exponential"
    initial = 6000.0 * 2 * math.pi / 60
    assert math.isclose(decay.spin_after(initial, 0.0), initial)
    # A ~4.5 s iron flight at 4%/s keeps e^-0.18 of its spin — enough to matter to the carry.
    assert math.isclose(decay.spin_after(initial, 4.5), initial * math.exp(-0.18))
    assert decay.spin_after(initial, 4.5) < initial


def test_reynolds_is_computable_even_though_the_lookup_does_not_use_it() -> None:
    """The stored Reynolds column is for a future 2-D read; the check it enables works today.

    A 7 iron at 90 mph of ball speed sits around Re = 118,000, inside the range the coefficients
    were measured over — which is worth knowing precisely because the *spin ratio* is not.
    """
    model = load_flight_model()
    speed_m_s = 90.7 * 0.44704
    reynolds = model.atmosphere.reynolds_for(speed_m_s, model.ball.diameter_m)
    measured = [row.reynolds for row in model.coefficients.rows]
    assert min(measured) < reynolds < max(measured)


def test_the_module_level_convenience_reads_the_same_table() -> None:
    table = load_flight_model().coefficients
    row = table.rows[-1]
    assert coefficients_for(row.spin_ratio).cl == row.cl


def test_the_artifact_is_loaded_once() -> None:
    """`lru_cache` over `importlib.resources`, as `joint.py` and `distributions.py` both do."""
    assert load_flight_model() is load_flight_model()
    assert flight_dataset_info() is flight_dataset_info()


# -------------------------------------------------------------------------------------------
# The atmosphere profile [M15 P6] — the altitude what-if, and why it is written as ratios
# -------------------------------------------------------------------------------------------

#: Denver, to the nearest metre: the altitude every golfer's rule of thumb is quoted at, and the
#: one this repo would reach for first if the bay ever moved. Tactu, Peru at 4369 m is the highest
#: course on earth and is here as the far end of the range rather than as a plan.
_DENVER_M = 1609.0
_TACTU_M = 4369.0


def test_the_profile_at_zero_altitude_returns_the_committed_air_to_the_bit() -> None:
    """The whole reason the profile is ratios and not the standard's absolute formulas.

    Exact by construction, not by the committed numbers agreeing with each other — which the test
    below measures them not entirely doing.
    """
    model = load_flight_model()
    sea_level = model.atmosphere
    at_zero = model.atmosphere_profile.at_altitude(sea_level, 0.0, model.gravity_m_s2)

    assert at_zero.density_kg_m3 == sea_level.density_kg_m3
    assert at_zero.kinematic_viscosity_m2_s == sea_level.kinematic_viscosity_m2_s
    assert at_zero.temperature_k == sea_level.temperature_k
    assert at_zero.pressure_pa == sea_level.pressure_pa

    # Equal numbers, deliberately unequal identity: a generated air state says which law generated
    # it rather than passing itself off as the published row.
    assert at_zero.name != sea_level.name
    assert at_zero.source == model.atmosphere_profile.source


def test_the_committed_sea_level_row_is_exact_in_density_and_rounded_in_viscosity() -> None:
    """The measurement behind the choice above, and it is only half the story people expect.

    ISO 2533 picked its gas constant so that `p0/(R*T0)` *is* the published 1.225 — to a relative
    1.5e-8, which is eight significant figures of agreement and nothing to worry about. The
    kinematic viscosity is the one that is rounded: Sutherland at 288.15 K gives 1.4607186e-5
    against a committed 1.4607e-5, a relative 1.3e-5.

    So an absolute profile would be exact at zero in one quantity and off in the fifth significant
    figure in the other, and which quantity that is depends on how the artifact was transcribed.
    That is the argument for ratios: they are exact in all four whatever the transcription did.
    """
    model = load_flight_model()
    profile = model.atmosphere_profile
    air = model.atmosphere

    absolute_density = air.pressure_pa / (profile.gas_constant_j_kg_k * air.temperature_k)
    assert absolute_density == pytest.approx(air.density_kg_m3, rel=1e-7)
    assert absolute_density != air.density_kg_m3, (
        "even the self-consistent one is not bit-exact, which is the whole point"
    )

    dynamic = (
        profile.sutherland_beta_kg_m_s_sqrt_k
        * air.temperature_k**1.5
        / (air.temperature_k + profile.sutherland_s_k)
    )
    absolute_viscosity = dynamic / air.density_kg_m3
    assert absolute_viscosity == pytest.approx(air.kinematic_viscosity_m2_s, rel=2e-5)
    assert abs(absolute_viscosity / air.kinematic_viscosity_m2_s - 1.0) > 1e-6, (
        "the viscosity is committed rounded to five figures; if it stops being, this test is "
        "where the artifact says so"
    )


def test_air_thins_and_cools_with_altitude_and_thickens_below_sea_level() -> None:
    model = load_flight_model()
    profile = model.atmosphere_profile
    sea_level = model.atmosphere

    above = [
        profile.at_altitude(sea_level, metres, model.gravity_m_s2)
        for metres in (0.0, 500.0, _DENVER_M, _TACTU_M)
    ]
    for lower, higher in zip(above, above[1:], strict=False):
        assert higher.density_kg_m3 < lower.density_kg_m3
        assert higher.pressure_pa < lower.pressure_pa
        assert higher.temperature_k < lower.temperature_k

    dead_sea = profile.at_altitude(sea_level, -390.0, model.gravity_m_s2)
    assert dead_sea.density_kg_m3 > sea_level.density_kg_m3
    assert dead_sea.temperature_k > sea_level.temperature_k


def test_the_profile_reproduces_the_standard_atmosphere_at_denver() -> None:
    """Against ISO 2533's own numbers, computed absolutely — the ratios are not a different model.

    Checked at a real altitude rather than only at zero, because the zero test is satisfied by any
    law at all: at `h = 0` every ratio here is exactly 1 by construction.
    """
    model = load_flight_model()
    profile = model.atmosphere_profile
    denver = profile.at_altitude(model.atmosphere, _DENVER_M, model.gravity_m_s2)

    temperature = model.atmosphere.temperature_k - profile.lapse_rate_k_per_m * _DENVER_M
    exponent = profile.pressure_exponent(model.gravity_m_s2)
    pressure = model.atmosphere.pressure_pa * (
        temperature / model.atmosphere.temperature_k
    ) ** exponent

    assert exponent == pytest.approx(5.25588, abs=1e-5)
    assert denver.temperature_k == pytest.approx(temperature)
    assert denver.pressure_pa == pytest.approx(pressure)
    assert denver.density_kg_m3 == pytest.approx(
        pressure / (profile.gas_constant_j_kg_k * temperature), rel=2e-5
    )
    # About 14.6% of the air gone a mile up, which is the figure the golf rule of thumb rests on.
    assert denver.density_kg_m3 / model.atmosphere.density_kg_m3 == pytest.approx(0.8544, abs=1e-4)


def test_kinematic_viscosity_rises_with_altitude_because_the_air_thins_faster() -> None:
    """The direction is the point: `nu = mu/rho`, and cold air is *more* viscous per unit mass.

    Nothing in the flight reads this today — the coefficient lookup is indexed on spin ratio — so
    the temptation is to leave it at its sea-level value. The day the stored Reynolds column gets
    a two-dimensional read, a frozen viscosity would be a silent error rather than a gap.
    """
    model = load_flight_model()
    profile = model.atmosphere_profile
    denver = profile.at_altitude(model.atmosphere, _DENVER_M, model.gravity_m_s2)

    assert denver.kinematic_viscosity_m2_s > model.atmosphere.kinematic_viscosity_m2_s
    # Dynamic viscosity falls with the temperature even as the kinematic one rises.
    assert profile.viscosity_ratio(denver.temperature_k, model.atmosphere.temperature_k) < 1.0
    # And a Reynolds number for the same ball at the same speed therefore falls at altitude.
    speed_m_s = 90.7 * 0.44704
    assert denver.reynolds_for(speed_m_s, model.ball.diameter_m) < model.atmosphere.reynolds_for(
        speed_m_s, model.ball.diameter_m
    )


def test_an_altitude_outside_the_layer_raises_rather_than_extrapolating() -> None:
    """R8's raising side: 30 km is a units bug, not a golf course, and there is nothing to say.

    The bounds are the standard's own layer rather than a judgement about golf — which is what
    makes them defensible — and they contain every course on earth by a wide margin.
    """
    model = load_flight_model()
    profile = model.atmosphere_profile
    assert profile.altitude_min_m <= -390.0
    assert profile.altitude_max_m >= _TACTU_M

    for metres in (profile.altitude_min_m - 0.1, profile.altitude_max_m + 0.1, 30_000.0):
        with pytest.raises(ValueError, match="outside isa_troposphere"):
            profile.at_altitude(model.atmosphere, metres, model.gravity_m_s2)


def test_swapping_the_air_leaves_the_cached_default_untouched() -> None:
    """`model_copy` on an `lru_cache`d singleton — the mutation this would be easy to write."""
    model = load_flight_model()
    before = model.atmosphere.density_kg_m3

    denver = model.at_altitude(_DENVER_M)

    assert denver is not model
    assert denver.atmosphere.density_kg_m3 < before
    assert load_flight_model().atmosphere.density_kg_m3 == before
    assert load_flight_model() is model, "the what-if must not evict the cache it read"
    # Everything but the air is the same object's data — the ball, the table and the decay rate
    # are properties of a golf ball rather than of where it is being hit.
    assert denver.ball == model.ball
    assert denver.coefficients == model.coefficients
    assert denver.spin_decay == model.spin_decay
    assert denver.atmosphere_profile == model.atmosphere_profile
