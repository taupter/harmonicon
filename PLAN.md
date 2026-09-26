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

- **Mastery meter recency.** Reviews now carry due dates
  (`TrainingRecord::review_due`), so the per-track mastery meter could
  discount tiers long overdue; today it counts cleared tiers only
  (`docs/training_tree_plan.md` §4).
- **Song Editor arranging**: repeats and endings, pickups (a chart starting
  mid-bar), lyrics, and batch editing beyond transposition. Transposition
  (`song_editor::transpose`) and the Record count-in already exist. Each
  remaining piece is a chart-schema addition — a `format_version` bump and
  a decision about what gameplay does with it — and needs round-trip and
  playability tests plus player and contributor docs as it lands.

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
