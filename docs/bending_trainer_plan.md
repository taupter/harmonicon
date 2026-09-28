# Bending Trainer: remaining validation

The control, drill, professional-control, and tablet implementation phases
are complete. The remaining work needs real harps and microphones; `PLAN.md`
tracks it.

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

Manual validation still matters. Test quiet and loud rooms, headphones and
speakers, all available pitch algorithms, landscape and portrait tablets, slow
bends, fast scoops, stable holds, vibrato, intentional overshoot, wrong holes,
and long silence. A professional player should specifically review whether
smoothing hides useful motion and whether the strict tolerance behaves
consistently across registers.

The harps on hand are a **C** and a **G**. The trainer puts a G harp an octave
*below* C (`harmonica::key_offset`: hole 1 blow is G3), matching a standard G.
Together they cover roughly 196 Hz to 2.1 kHz: the G supplies the low end,
including its hole 2–3 draw bends, and the C supplies the top. Two registers
remain unverified until someone with the harps checks them:

- **Below G3 (low harps, down to ~131 Hz on a low C).** This is where the
  detectors have the least frequency resolution to work with, so tolerance and
  stability are most likely to misbehave here. The G harp's lowest holes are
  the nearest available evidence; treat a clean result there as encouraging,
  not conclusive.
- **Above C7 (high harps, up to ~3.1 kHz on a high G).** This covers the top
  blow bends and overdraws of the high keys. The C harp's holes 8–10 are the
  nearest available evidence.

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
