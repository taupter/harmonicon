# Plan

Open work only, gathered from every plan in `docs/` and checked against the
code. Each item names the doc holding its design. When something lands,
delete its line: git history is the record of what shipped, and `CLAUDE.md`
holds any invariant future code must respect. Companions: `TODO.md` (the
small-item checklist) and `ROADMAP.md` (the destination).

## Needs a harp, a microphone, or ears

Nothing below can be approved by a test.

- **Bending Trainer validation**, with the C and G harps on hand:
  `docs/gameplay_validation.md` § Bending Trainer. Registers neither harp
  reaches (below G3, above C7) stay open in `docs/bending_trainer_plan.md`
  § Verification.
- **Jam Session listening pass**, the only work left in
  `docs/jam_feel_plan.md`. Run `cargo run -p harmonicon-jam --example
  listening_matrix`, then complete the Jam Session rows in
  `docs/gameplay_validation.md`: balance, genre credibility, the call's
  timbre and duck depth, the thinning depth, and whether the band's answers
  sit in each groove. The outcome must be rows marked complete or named mix
  changes, not impressions.
- **Scored play, live**: the mid-hold drop-out, the steady-vs-wobble vibrato
  contrast, and whether judgment feedback lands on the same visual beat as
  the judged note (`docs/gameplay_improvement_plan.md`, Phase 2).
- **A recorded detection corpus** (`docs/pitch_detection_plan.md`): single
  notes, bends, overbends, adjacent-hole chords, octave splits, tongue-block
  intervals, blow/draw transitions, breath-only passages and room noise, at
  several loudnesses, distances and, where practical, two microphones. Keep
  the first takes fixed as the baseline. Once it exists:
  - add detector strength to `PitchInfo` and infer direction from summed
    evidence;
  - tune onset, release and direction-change hysteresis from measured
    errors, including whether `release_frames` should rise above 1;
  - learn per-(hole, direction, technique) spectral templates and fit them
    with sparse non-negative reconstruction plus noise and residual terms;
  - add a frequency-dependent noise estimate and separate attack/sustain
    thresholds;
  - decide whether the harmonica constraint solver goes live, scoped to
    NMF only. It is proven only on synthetic audio, and a real reed's
    blow/draw transition may be stripped like a phantom.

  Accept a change only if the corpus improves without losing exact chord
  recall or adding latency.

## Code work, unblocked

- **Training tiers state their goal.** A tier button reads only `1`–`5`.
  Show the tier's name (Isolate … Interleave) and its concrete goal before
  it starts; `training_criteria` already computes it
  (`docs/training_tree_plan.md` §4).
- **Practice motivation**, in this order: a per-track mastery meter (only
  the per-node ring exists), a spaced "Warm-up" review queue from
  last-passed dates, then a practice streak that forgives a missed day and
  is never framed as loss (`docs/training_tree_plan.md` §4).
- **Measure the curriculum's chokepoints.** `lessons::graph::min_choices`
  has only ever run on synthetic graphs; the chokepoints were measured on
  the old 41-lesson curriculum. Run it on the shipped 100 lessons, and where
  one lesson is still the only way forward, widen the prerequisites
  (`docs/training_tree_plan.md` §1).
- **`note_bench` metrics**: exact-set chord precision/recall, per-note
  precision/recall, direction accuracy, onset and release latency, and
  per-scenario summaries. Buildable now against the synthetic dataset;
  meaningful once the corpus exists (`docs/pitch_detection_plan.md`).
- **Layout assertions** for scored play's pure decisions, such as which
  contextual panels a chart shows (`docs/gameplay_improvement_plan.md`,
  Phase 0).
- **Song Editor arranging**: repeats and endings, pickups/count-in, lyrics,
  transposition and batch editing. Each needs round-trip and playability
  tests plus player and contributor docs as it lands.

## Decide before building

- **Are generated trainings good enough?** The generator exists and the
  `bend` track has specs (`first-bend`, `deep-bends`, `high-blow-bends`).
  Play them. Only a yes rolls trainings out to the other tracks
  (`docs/training_tree_plan.md`, Order of work 1 and 5).
- **Technique symbols beside the notes.** The note-head label drops the
  bend/overblow/slide suffix on purpose, so this reverses a design rather
  than fixing an oversight (`docs/gameplay_improvement_plan.md`, Phase 1).
- **Play 3D's hole map and beat guides.** Its lane is world-space geometry,
  so guides mean projecting `HIT_Z` per beat, not reusing the 2D spawner
  (same phase).
- **The single-lesson `hand` track**: fold it into `tone`, or leave the gap
  visible as a place the curriculum wants more lessons.

## Deferred until a condition is met

- **A graded `Sustain` outcome**: only as a deliberate scoring change,
  never folded into UI work.
- **Judgment sounds**: only after testing against microphone capture,
  since speaker feedback can contaminate detection.
- **A lesson-map minimap**: only on usability evidence.

## Content, not to be authored unsupervised

- **Blues starter pack** (1.0 rc3). The automatable half is validation and
  difficulty calibration.
- **Recorded backing loops** per style (shuffle, slow blues, swing), as an
  alternative to the generated band.

## Release (1.0, desktop)

- Flathub submission and release signing keys. Version/tag agreement is
  already enforced by `release.yaml`.

## Mobile (post-1.0, needs hardware)

`contributing/src/android-build.md` has the detail.

- Run on a real phone or tablet: does the mic capture usably, and what are
  its latency and AGC like?
- Persist progress and settings on Android: `dirs::config_dir()` is `None`
  there, so both are lost on exit.
- A touch and hit-target pass, re-tuning `CompactLayout` against a real
  device rather than a small desktop window.
- An app icon, ABIs beyond arm64, and a release key in place of the debug
  one.
- iOS is untouched and needs Xcode.
