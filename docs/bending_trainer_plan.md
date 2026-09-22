# Bending Trainer: control-first improvement plan

## Product boundary

The Bending Trainer is a deliberate-practice tool for controlling one reed
interaction at a time. It should help a player hear, find, hold, release, and
repeat a bent or overbent pitch. It is not a lesson sequence and should not
explain a curriculum, unlock content, or issue pass/fail grades. Lessons own
teaching order and progression; Jam Session owns using bends while making
music.

There is currently no separate bending-trainer crate. The feature lives in
`harmonicon-gameplay::gameplay::bending_trainer`, with persisted drill records
in `harmonicon-app::profile`. This plan keeps that dependency direction.

The screen must work well on tablets. A phone-specific layout remains out of
scope.

## What exists today

The current trainer has a useful foundation:

- a transposed ten-hole Richter diagram with directly selectable natural,
  bend, overblow, and overdraw cells;
- selectable harp key and pitch-detection algorithm, with the detector range
  rebuilt for the chosen harp;
- a clean synthesized reference pitch and a live cents-flat/cents-sharp tuner;
- technique hints that distinguish draw bends, blow bends, overblows, and
  overdraws;
- a 40–220 BPM metronome;
- an adaptive drill that asks for playable targets, accepts a pitch held
  within six cents for 0.5 seconds, times out after 12 seconds, and weights
  future picks toward lower lifetime hit rates;
- per-hole/technique persistence and a colored progress map;
- pure helpers and tests for target resolution, valid target sets, weighting,
  persistence, hints, pitch synthesis, and progress colors.

The experience is less capable than that list suggests:

1. Feedback shows only the distance from the final target. A new player cannot
   see the natural starting pitch, the direction of travel, whether the bend
   is moving smoothly, or whether they overshot into the next bend slot.
2. The drill mixes natural notes, ordinary bends, deep bends, and overbends in
   one pool. That can give a novice an overdraw before they can control hole 2
   draw, while an expert cannot isolate overblows, high blow bends, or one weak
   hole.
3. Every player gets the same six-cent tolerance, half-second hold, and
   12-second timeout. These are plausible defaults, not a complete practice
   model.
4. A timeout is recorded as a miss even if the player never attacked the note.
   Walking away, adjusting the microphone, or taking a breath damages the
   progress map. Lifetime hit rate then makes early failures linger forever.
5. A binary hit does not describe bend control. Entry time, overshoot,
   stability, sustained center, release, and repeatability matter to serious
   players; the trainer records none of them.
6. The metronome is visually present but does not shape an exercise. It cannot
   ask for a bend on a beat, a controlled hold, a return to the natural note,
   or repeated pulses.
7. Reference targets assume A4 = 440 Hz and equal-tempered theoretical reeds.
   Real harps vary by tuning, key, reed setup, temperature, and playing
   pressure. This is acceptable for orientation but limiting for precision
   work.
8. The progress map relies on red-to-green color and one lifetime percentage.
   It does not distinguish unattempted, inconsistent, slow-to-find, and stable
   targets without color.
9. Several player-facing target labels and physical-technique hints are
   hard-coded English even though the rest of the screen uses Fluent.
10. The overbend hint prescribes a tongue-blocked embouchure. Overblows and
    overdraws can be played with different embouchures; the trainer should
    describe the reed/airway goal without presenting one mouth position as a
    requirement.

## Two players, one progressively disclosed tool

### Novice needs

A novice needs trust before difficulty: confirmation that the microphone hears
the correct hole, a clear natural-to-target path, a forgiving stability window,
small target sets, and an obvious way to replay the reference. Silence and a
wrong hole should be described differently. The trainer should celebrate
control without turning a practice attempt into a grade.

The first useful workflow is: play the natural note, lower it toward the marked
target, hold it briefly, then release cleanly. Hole 2 and hole 3 draw bends are
more useful starting material than a random tour of every technique.

### Professional needs

An experienced player needs control over the practice constraint: exact hole
and technique sets, tolerance, hold duration, tempo, reference pitch, and
whether the exercise measures entry, sustain, release, or vibrato. They need a
pitch trace and recent consistency, not a large “correct” indicator. They may
want to compare harp keys or individual instruments whose real reed centers do
not match a theoretical A440 target exactly.

These needs should appear through an Advanced drawer and custom drill setup.
The default screen should remain immediate enough for a first-time player.

## Implementation order

### 1. Make the target and microphone state trustworthy

**In progress.** Technique copy is localized, overbend guidance no longer
requires one embouchure, live and drill feedback are locked to the selected
hole's pitch family, and the optional natural-note check confirms a centered
reed attack. Temporal detector-confidence tracking and the explicit unstable
state remain for the next slice.

Fix ambiguity before adding more drills.

- Localize technique names, target labels, and physical hints through Fluent.
- Rewrite bend and overbend hints around airflow, tongue position, and gentle
  pressure without requiring a specific embouchure. Keep safety language
  concise: force is not the route to a deeper bend.
- Replace the single tuner sentence with distinct states: no microphone/no
  signal, wrong playable pitch, approaching the target, centered, and unstable.
- Lock feedback to the selected hole's playable pitch family rather than the
  globally closest detected frequency. Reject octave errors and unrelated
  harmonics before computing cents.
- Add an input-readiness check using a natural note on the selected hole. The
  check should confirm signal tracking and establish that reed's observed
  center; it should not be a mandatory calibration wizard.
- Surface detector uncertainty through a quiet “signal unstable” state rather
  than allowing noisy frames to advance a drill.

Exit criteria: a player can tell the difference between silence, the wrong
hole, a noisy estimate, and a real attempt; a natural-note check cannot be
mistaken for a successful bend.

### 2. Show the bend as motion, not a verdict

Build a `BendTrace` from timestamped cents relative to the selected hole's
natural reed and target.

- Make the central visual a vertical or horizontal pitch rail from natural
  note to target, with named intermediate bend slots where they exist.
- Draw the live pitch continuously, including overshoot beyond the target, and
  retain only the last few seconds as a fading trace.
- Show three separate qualities: distance to target, stability while held, and
  time held. Do not collapse them into one score.
- Mark the target band with the current tolerance. Use shape, labels, and line
  style as well as color.
- Keep the numeric cents readout available. It is supporting detail for a
  novice and essential detail for a professional.
- Let Listen alternate the natural anchor and target, with separate buttons or
  an A/B action, so the player hears the interval they are trying to create.

Exit criteria: without reading prose, a player can see where the bend started,
which direction it moved, whether it crossed the target, and whether the held
pitch settled.

### 3. Add focused practice shapes

Keep free exploration as the default and make structured practice explicit:

- **Find and hold:** enter the target from silence and hold it.
- **Bend and release:** establish the natural note, move to the target, hold,
  and return to the natural note without a pitch jump.
- **Repeated bends:** perform the same path on successive metronome beats.
- **Bend ladder:** move through each valid bend depth on one hole in order and
  back out, useful for holes 2 and 3.
- **Overbend response:** natural attack, clean overbend onset, hold, release;
  available in advanced scope rather than the novice default.

Represent these as small pure state machines driven by pitch frames and the
shared metronome clock. A mode describes the requested gesture; it does not
become a lesson or unlock gate.

Exit criteria: the metronome changes what the player performs, and every
gesture can be replayed deterministically from a recorded pitch stream.

### 4. Give the adaptive drill an honest scope and memory

- Add drill scopes: **First bends**, **All bends**, **Blow bends**,
  **Overbends**, and **Custom**. Start novices with a small set of common draw
  bends; never introduce overbends through the default scope.
- Let Custom select individual diagram cells and optionally one practice shape.
- Count an attempt only after a credible onset in the selected hole's pitch
  family. A 12-second period with no attempt becomes “skipped,” not “missed.”
- Replace lifetime hit-rate weighting with recent evidence: keep attempts,
  successful controls, skips, recent stability, and last-practiced time. Use a
  bounded moving estimate so old beginner failures gradually stop dominating.
- Prevent target churn after a noisy frame. Advance only after the practice
  shape reaches a terminal success/skip state.
- Migrate existing `DrillRecord { attempts, hits }` data without losing it.
  Keep storage types in `harmonicon-app`; gameplay converts them into its richer
  internal model.

Exit criteria: leaving the instrument down does not lower progress, novice
scope contains no overbends, and weak or stale targets return without trapping
the player in a permanent low percentage.

### 5. Add professional controls without crowding the default

Put these in an Advanced drawer remembered across visits:

- target tolerance presets (for example 12, 6, and 3 cents) plus a bounded
  custom value;
- hold duration and attempt timeout;
- A4 reference and a per-harp/per-hole natural-center offset captured by the
  readiness check;
- practice-shape tempo and subdivisions;
- trace duration and smoothing;
- a stability view showing mean cents, pitch spread, and longest centered
  hold for the current attempt;
- an optional vibrato window showing rate and depth after the centered bend is
  established.

Do not infer breath pressure, embouchure quality, or reed health from pitch
alone. The microphone does not contain enough evidence for those claims.

Exit criteria: an expert can define a repeatable precision exercise while a
new player can ignore every advanced control.

### 6. Rebuild the tablet interaction hierarchy

The current equal-width columns give the diagram half the screen even when the
pitch trace is the active task.

- Use a calm three-part hierarchy: compact setup/transport across the top,
  selected-hole bend path as the large center, and the full harp diagram as a
  target picker/reference below or beside it according to tablet orientation.
- Keep the primary actions near the pitch path: Natural, Target, Start/Stop
  drill, and Skip.
- Put detector selection, key, advanced controls, and the full progress map in
  drawers that do not move the live trace when opened.
- Make every selectable diagram cell keyboard/focus accessible and give it a
  text/shape progress state in addition to tint.
- Preserve a large landscape tablet layout and a usable portrait tablet
  layout. Do not spend this pass on narrow phone breakpoints.

Exit criteria: the live pitch path is the visual center, the current action is
reachable without crossing the screen, and all controls remain usable without
hover.

## Architecture

- Keep `bending_trainer/mod.rs` responsible for lifecycle and screen
  composition.
- Keep target enumeration and persisted adaptive selection in `drill.rs`, but
  split gesture evaluation into `gesture.rs` and trace/history math into
  `trace.rs` before either grows large.
- Model detector input as a small neutral frame containing time, frequency,
  playable-family match, and stability. Pure state machines consume frames;
  Bevy systems only translate resources and update views.
- Reuse `harmonicon-core` pitch maps and harmonica layouts as the authority for
  natural, bent, and overbent targets. Do not duplicate Richter note tables.
- Keep reference synthesis in the trainer until another feature needs the same
  natural/target A/B voice.
- Persist configuration separately from performance history. Preferences such
  as tolerance and trace length belong with settings; per-target evidence
  belongs with the player profile.

## Verification

Unit tests should cover:

- pitch-family locking, octave rejection, readiness offsets, and unstable
  input;
- trace bounds, target crossings, overshoot, stability, and natural-to-target
  normalization;
- every gesture state transition under recorded pitch streams, including
  silence and wrong-hole interruptions;
- drill-scope membership for every Richter hole and technique;
- attempt versus skip semantics, recent-evidence weighting, migration from the
  existing profile shape, and deterministic selection with a supplied seed;
- reference-pitch and per-hole-offset math;
- localization-key parity and non-color progress labels.

Manual validation still matters. Use at least a low C, C, and high G Richter
harp; test quiet and loud rooms, headphones and speakers, all available pitch
algorithms, landscape and portrait tablets, slow bends, fast scoops, stable
holds, vibrato, intentional overshoot, wrong holes, and long silence. A
professional player should specifically review whether smoothing hides useful
motion and whether the strict tolerance behaves consistently across registers.

## Deliberately out of scope

- lesson sequencing, explanatory chapters, pass/fail certification, or
  unlocking techniques;
- judging bends inside songs or Jam Session;
- microphone monitoring through speakers;
- claims about breath pressure, embouchure correctness, reed condition, or
  player health from pitch alone;
- automatic harp-model recognition;
- phone-specific layout work;
- chromatic-harmonica slide training, which is a different physical gesture
  and deserves its own practice design if added later.
