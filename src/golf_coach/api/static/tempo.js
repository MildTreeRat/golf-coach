// The tempo trainer's controls, shared by results.html and career.html. [ADR-023]
//
// This file exists because career mode grew a second trainer (ADR-023's addendum, 2026-08-22) and
// a second copy of a Web Audio scheduler is a second thing that drifts — the failure this repo has
// already been bitten by three times in prose and once in a parser. It is a plain <script src>
// with no build step and no module system, which is the same choice `REFACTOR_LEDGER.md` records
// for the pages themselves: a local-first tool installed with `pip install -e .` does not get a
// node toolchain to keep working.
//
// **What lives here is the mechanism. What does not is the framing.** The two pages genuinely say
// different things — one is answering "this swing was quick", the other "this is what you usually
// do" — so the heading, the verdict prose and the anchor sentence are each page's own and are
// passed in. Only the beats, the audio, the strip and the pace control are shared, because those
// are identical by construction: both come from one `TempoPlan` built by one
// `analysis/tempo_trainer.py`.
//
// **No duration, ratio, tick count or mode name is written in this file.** Every one of them is
// derived server-side from the reference distributions and arrives on the plan. That is the
// standing rule the placements block is pinned to, and the reason both patterns are computed there
// rather than one being derived here from the other: GRID's ratio is a *rounded* one, and a page
// recomputing it from CUES would silently print a number the golfer is not hearing.

const TempoTrainer = (() => {
  // Everything below is presentation of a payload, with one exception worth naming: the pitch
  // table. A role needs a *sound*, and a sound is not a fact about the swing — sending frequencies
  // from the server would be absurd. So roles are named here, and only to be pitched and captioned.
  //
  // `subdivision` is quieter and lower than every cue on purpose. It marks nothing; it exists to
  // keep the pulse audible through the ~900 ms the backswing takes, and a golfer who hears it as a
  // cue would be swinging to the wrong beat.
  const BEAT_TONE = {
    takeaway:    { hz: 660, gain: 0.30 },
    top:         { hz: 880, gain: 0.30 },
    impact:      { hz: 1320, gain: 0.34 },
    subdivision: { hz: 330, gain: 0.10 },
  };

  // How long each click sounds for. Short — a metronome tick, not a note.
  const BEAT_SECONDS = 0.045;
  // Count-in ticks before the first beat, and rest ticks between loops. Both measured in the
  // pattern's *own* interval, so they stay in proportion at any pace and in either mode.
  const COUNT_IN_TICKS = 2;
  const REST_TICKS = 2;

  let state = null;
  let audio = null;
  let timer = null;

  function esc(value) {
    return String(value).replace(/[&<>"']/g, (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
  }

  function num(value, digits = 2) {
    if (typeof value !== "number") return "-";
    const fixed = value.toFixed(digits);
    return fixed.includes(".") ? fixed.replace(/0+$/, "").replace(/\.$/, "") : fixed;
  }

  // The markup, ids and all. Returned rather than injected so each page decides what wraps it.
  //
  // The pace bounds are the anchor guard's own range expressed as a percentage of the tour median,
  // and `tests/analysis/test_tempo_trainer.py` reads them back out of this file: a guard widened
  // without widening the control would clamp the opening value and quietly play a tempo other than
  // the one the page's own text claims. Written here rather than on each page so there is one pair
  // of numbers to keep true, and one place for that test to look.
  function markup(plan) {
    if (!plan || !(plan.patterns || []).length) return "";
    const modes = plan.patterns.map((p, i) =>
      `<button type="button" data-mode="${i}" aria-pressed="${i === 0}">${esc(p.mode)}</button>`
    ).join("");

    return `<p>Swing along with the beats: <strong>first beat</strong> start the takeaway,
      <strong>top</strong> is the top of the backswing, <strong>impact</strong> is the ball.
      Two count-in ticks come first, then it loops.</p>
    <div class="modes">${modes}</div>
    <div id="tempoStrip"></div>
    <div class="controls">
      <button type="button" class="play" id="tempoPlay">Play</button>
      <label>Pace <input type="range" id="tempoPace" min="70" max="140" step="1" />
        <span id="tempoPaceOut"></span></label>
    </div>
    <p class="facts" id="tempoFacts"></p>
    <p class="anchor" id="tempoAnchor"></p>
    <p class="src" id="tempoSrc"></p>`;
  }

  // `anchorText` is the caller's, and required: it is the sentence saying which backswing the
  // target was built on, and the two pages have different numbers of answers to that. The results
  // page has two (this swing's, or the tour median); career mode has three, because a golfer whose
  // mean is still withheld is anchored to their latest swing and is in neither of the other states.
  function wire(plan, { anchorText }) {
    const play = document.getElementById("tempoPlay");
    if (!plan || !play) { state = null; return; }

    // The slider starts where the server fitted it, not at a neutral 100%. `plan.pace` is a
    // multiple of the *tour median* backswing, so a golfer whose own backswing is 11% longer opens
    // at 111% — the control shows the decision rather than hiding it, and dragging is an override.
    //
    // `pace` is clamped rather than trusted, because a future guard could widen and a slider that
    // cannot reach its own starting value would silently play something other than what it says.
    const pace = document.getElementById("tempoPace");
    const opening = Math.min(Number(pace.max), Math.max(Number(pace.min),
      Math.round((plan.pace || 1) * 100)));
    pace.value = String(opening);
    state = { plan, mode: 0, pace: opening / 100, anchorText };
    document.getElementById("tempoPaceOut").textContent = `${opening}%`;

    document.querySelectorAll(".modes button[data-mode]").forEach((btn) => {
      btn.addEventListener("click", () => {
        state.mode = Number(btn.dataset.mode);
        document.querySelectorAll(".modes button[data-mode]").forEach((b) =>
          b.setAttribute("aria-pressed", String(b === btn)));
        // Switching mode or pace rebuilds from the payload rather than rescaling what is playing:
        // the two patterns have different beat counts, so there is no correspondence to preserve.
        restartIfPlaying();
        draw();
      });
    });

    pace.addEventListener("input", () => {
      state.pace = Number(pace.value) / 100;
      document.getElementById("tempoPaceOut").textContent = `${pace.value}%`;
      restartIfPlaying();
      draw();
    });

    play.addEventListener("click", () => (timer ? stop() : start()));
    draw();
  }

  function currentPattern() {
    return state.plan.patterns[state.mode];
  }

  // Beat times at the pace the golfer chose. The payload's own `pace` is what the server already
  // applied, so the two multiply rather than one overriding the other.
  function pacedBeats() {
    const scale = state.pace;
    return currentPattern().beats.map((b) => ({ role: b.role, at_ms: b.at_ms * scale }));
  }

  function pacedInterval() {
    return currentPattern().downswing_ms * state.pace;
  }

  function draw() {
    const pattern = currentPattern();
    const beats = pacedBeats();
    const span = beats[beats.length - 1].at_ms || 1;

    // Bars separated by proportional gaps, so the strip is a picture of the real intervals.
    let strip = "";
    beats.forEach((b, i) => {
      if (i) strip += `<span class="gap" style="flex-grow:${(b.at_ms - beats[i - 1].at_ms) / span}">
        </span>`;
      strip += `<span class="b ${esc(b.role)}"></span>`;
    });
    document.getElementById("tempoStrip").innerHTML = `<div class="strip">${strip}</div>
      <div class="legend"><span>takeaway</span><span>top</span><span>impact</span></div>`;

    const plan = state.plan;
    const observed = (plan.observed_backswing_ms && plan.observed_downswing_ms)
      ? ` &middot; yours ${num(plan.observed_backswing_ms, 0)} / ${
          num(plan.observed_downswing_ms, 0)} ms (${
          num(plan.observed_backswing_ms / plan.observed_downswing_ms, 2)}:1)`
      : "";
    document.getElementById("tempoFacts").innerHTML =
      `${num(pattern.backswing_ms * state.pace, 0)} ms back / ${
        num(pattern.downswing_ms * state.pace, 0)} ms down &middot; ${
        num(pattern.ratio, 2)}:1${observed}`;
    // Which backswing the target was built on. A golfer swinging at 90 mph and one at 110 do not
    // share an absolute tempo — the corpus puts a whole speed cohort 167 ms apart on the backswing
    // — so the trainer fits to theirs where it can, and has to say when it could not.
    document.getElementById("tempoAnchor").textContent = state.anchorText;
    document.getElementById("tempoSrc").textContent = pattern.source;
  }

  function start() {
    const Ctx = window.AudioContext || window.webkitAudioContext;
    if (!Ctx) return;
    if (!audio) audio = new Ctx();
    // iOS starts every context suspended until a gesture resumes it; this runs inside a click.
    if (audio.state === "suspended") audio.resume();

    document.getElementById("tempoPlay").textContent = "Stop";
    scheduleCycle();
  }

  // One cycle: the count-in, the pattern, then a rest, and re-arm. Scheduled a cycle at a time
  // through the Web Audio clock rather than a click per `setTimeout` — `setTimeout` drifts by tens
  // of milliseconds under load, which on a 267 ms downswing is a quarter of the interval.
  function scheduleCycle() {
    const beats = pacedBeats();
    const tick = pacedInterval();
    const startAt = audio.currentTime + 0.12;
    const countIn = COUNT_IN_TICKS * tick;

    for (let i = 0; i < COUNT_IN_TICKS; i++) {
      click(startAt + (i * tick) / 1000, BEAT_TONE.subdivision);
    }
    for (const b of beats) {
      click(startAt + (countIn + b.at_ms) / 1000, BEAT_TONE[b.role] || BEAT_TONE.subdivision);
    }

    const cycleMs = countIn + beats[beats.length - 1].at_ms + REST_TICKS * tick;
    timer = setTimeout(scheduleCycle, cycleMs);
  }

  function click(at, tone) {
    const osc = audio.createOscillator();
    const gain = audio.createGain();
    osc.frequency.value = tone.hz;
    // Ramped rather than switched: a square-edged gain change is an audible pop on every tick.
    gain.gain.setValueAtTime(0, at);
    gain.gain.linearRampToValueAtTime(tone.gain, at + 0.005);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + BEAT_SECONDS);
    osc.connect(gain).connect(audio.destination);
    osc.start(at);
    osc.stop(at + BEAT_SECONDS + 0.02);
  }

  function stop() {
    clearTimeout(timer);
    timer = null;
    const play = document.getElementById("tempoPlay");
    if (play) play.textContent = "Play";
  }

  function restartIfPlaying() {
    if (!timer) return;
    clearTimeout(timer);
    scheduleCycle();
  }

  return { markup, wire, stop };
})();
