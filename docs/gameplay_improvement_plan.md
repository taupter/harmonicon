# Gameplay Experience Improvement Plan

The gameplay clock, note scheduling, and scoring rules are already the strongest
parts of scored play. The roughness is concentrated in what the player sees and
learns from them. The current screen gives a large share of its space to song
metadata, a generic twelve-bar grid, and a complete technique legend, while the
time-critical area around the hit line gives only `PERFECT` or `GOOD` after a
successful onset. A miss, a wrong pitch, a failed technique, and a broken
sustain can all feel much the same.

This work should improve the visible feedback loop without changing the clock,
hit windows, pitch filter, or score calculation at the same time. Those systems
have good automated coverage and real-audio timing constraints; combining a UX
redesign with scoring changes would make regressions hard to diagnose.

## Product goals

During a song, a player should be able to answer four questions without looking
away from the hit line:

1. What should I play now?
2. What did Harmonicon hear?
3. Was my attack early, on time, or late?
4. If the note failed, was the problem pitch, timing, technique, or sustain?

The default layout should prioritize the next few notes and this feedback.
Contextual teaching aids should appear only when the chart contains the data
that makes them meaningful. Results should turn the run into one useful next
step instead of presenting a ledger of counters.

## Phase 0: establish a visual baseline — done

`python3 scripts/brpctl.py --take-all-screenshots` drives Play 2D (three chart
fixtures), Play 3D, pause, wait-for-note and results at **1920×1080, the
supported floor**, writing named PNGs to `target/screenshots/tour/`. Findings
are in `docs/gameplay_baseline.md`; re-run it for the after half of any
comparison.

There is no smaller desktop size to design for — phones ship 1080p panels.
Narrower than Full HD is Android portrait, and that wants its own baseline
once anyone has run the APK on hardware, not a shrunken desktop one.

Two things it does not cover, and why:

- **A scripted mixture of hits and misses.** `ActivePitches` isn't reflected,
  so detected pitches can't be injected over BRP. Hits in a captured run come
  from whatever the microphone actually hears. The judgment vocabulary itself
  is covered headlessly instead (`gameplay::tests::score_notes_blames_*`).
- **Layout assertions.** Still worth adding for pure decisions such as which
  contextual panels are visible; rendering quality stays screenshot/manual.

## Phase 1: make the highway the visual focus

Landed so far:

- `hud::spawn_score_readout` is one shared score/combo/judgment readout,
  anchored at each mode's own hit line instead of a screen corner.
- `song_info` moved key, harp, description and author to the countdown and the
  pause menu, leaving a title header — which gave the 2D highway 60% → 74% of
  the width.
- Beat and downbeat guides scroll the 2D highway (`gameplay/beat_guides.rs`),
  placed through the chart's tempo map and the notes' own
  `note_head_bottom_pct` rather than a second timing calculation.
- The technique legend lists only techniques the chart actually uses.
- The hit line spans the full highway (it stopped at 80%), and is warmer and
  heavier than a beat guide so the two can't be confused as they scroll past.
- Breath direction was already readable without colour and needed nothing: a
  note head reads `+7`/`-8`, or `↑`/`↓` when hole numbers are off, and the
  hole strip derives its lanes from the same `100.0 / hole_count` the highway
  does, so the numbers already line up under their lanes.

- Play 3D shares the panel: `hud_panel::spawn_hud_panel` is one spawner both
  modes call, on the same side, in the same order, off the same
  `used_modifiers` reading of the chart.

Still open:

- Keep technique symbols close to the notes that use them; the legend is
  supporting help, not permanent primary content. Note that the note-head
  label deliberately drops the bend/overblow/slide suffix today — that detail
  lives in the tab ribbon — so this is a real design reversal, not an
  oversight.
- 3D still has no hole map, and no beat guides: its lane is world-space
  geometry, so guides there mean projecting `HIT_Z` per beat rather than
  reusing a UI percentage. That is a different technique, not a shared
  spawner, and worth deciding deliberately.

Acceptance: at 1920×1080, the next notes, hit line, expected hole/direction,
score, and pause control are all legible without overlap. A non-blues chart
shows no invented blues form. *(The invented blues form is gone; the hit-line
and 3D-parity work is open.)*

## Phase 2: report what happened at each note

The message contract is in place. `NoteScored` carries a `JudgmentFeedback`
(`gameplay/state.rs`) rather than a bare `HitQuality`: `Hit { quality, offset }`
with the scorer's own signed offset, `Miss(MissReason)` with a `NoAttack` /
`WrongPitch` / `IncompleteChord` category, or `TechniqueMiss` when a declared
sustained technique is not confirmed at the end of a hold. Attribution
accumulates on `ScheduledNote::miss_evidence` while the note is pending, from
the same frame and `PitchGate` state `score_notes` judges from — see the crate's
`CLAUDE.md`. The HUD renders the value and classifies nothing itself.

A graded `Sustain` outcome (onset landed, note not held for enough of its
duration) is still deliberately absent: sustain currently scales points
continuously rather than passing or failing, and making it an outcome is a
scoring change, which this work is keeping separate.

A wrong-pitch miss captions itself with the expected and heard tabs
(*wanted 4↑ · heard 4↓*) on a second HUD line, blank for every other judgment.
The heard pitch is resolved to a hole in the judge against `PlayedHarp`, not in
the HUD.

Remaining in-play presentation work:

- animate the judged note itself and reset cleanly across A–B loops;
- show hold progress on sustained notes and confirm or reject vibrato/wah while
  the hold is still happening, not only once it ends;
- keep effects short and spatially stable so dense passages do not produce a
  wall of labels.

Acceptance: pure/headless tests assert the emitted feedback for perfect,
early/late good, no-attack miss, wrong-pitch miss, incomplete chord, and failed
sustained technique *(done — `gameplay::tests::score_notes_blames_*` and
siblings)*. Live validation confirms that feedback appears on the same visual
beat as the judged note.

## Phase 3: make practice controls part of the loop

- Turn wait-for-note into a clear coaching state at the hit line: show the
  expected tab and a live heard-pitch indicator while frozen, then give an
  immediate success transition when play resumes.
- Make A–B loop handles and the selected phrase visible without requiring the
  player to infer them from a thin progress strip. Keep range editing available
  while paused, where precise dragging is easier.
- Replace the pause menu's flat collection of controls with three groups:
  session actions, playback aids, and phrase practice. Hide phrase controls when
  the chart has no phrases and adaptive difficulty is unavailable.
- Surface practice speed and wait-for-note state in the live HUD with compact
  badges, so resuming does not make the player forget why audio is muted or the
  highway has stopped.
- Preserve the existing immediate resource updates and sink-safe rewind path;
  this is a presentation and navigation change around working mechanics.

Acceptance: a player can select a phrase, slow it down, loop it, enable waiting,
resume, and understand every active aid from the live screen. Restart and quit
remain visually separated from practice adjustments.

## Phase 4: turn results into coaching

- Lead with accuracy and the best actionable observation, then show grade and
  score. Examples: consistently late timing, weak bend accuracy, or many missed
  attacks. Generate this from `SongStats` with deterministic pure functions.
- Remove redundant rows. `Hits` is the sum of perfect/good/delayed and does not
  need equal visual weight beside those components.
- Present timing as a small centered distribution or early/on-time/late bar,
  with the existing latency adjustment action only when the evidence is strong
  enough. A mean alone can hide a wide, inconsistent distribution, so retain a
  bounded set of per-hit offsets during a run or add histogram buckets.
- Rank technique rows by the practice opportunity they reveal, while still
  showing sample counts. Do not recommend work from one attempted note.
- Add `Retry`, `Practice missed section`, and `Continue`. The practice action
  should choose the densest missed phrase or a bounded range around misses and
  enter the existing loop/practice machinery rather than create another mode.
- For lessons, put pass/fail criteria and progress toward the threshold above
  general song score.

Acceptance: pure tests cover coaching-message selection, minimum sample sizes,
timing histogram buckets, and missed-range selection.

## Phase 5: polish and accessibility

- Finish the colorblind-safe note palette work already listed in `PLAN.md`.
  Breath direction is already covered by the note-head label (see Phase 1), so
  what is left is **hit state**: `gameplay_2d::note_tint` signals hit and miss
  with gold and dim red and nothing else, which is the one place on the
  highway where colour is still the sole cue.
- Add reduced-motion and feedback-intensity settings before introducing camera
  shake or large pulses. Visual motion must never move the hit target.
- Audit text contrast, focus order, touch target size and localization
  expansion. Window size below Full HD is explicitly *not* in scope for
  desktop; `CompactLayout` earns its keep on Android portrait, and should be
  re-tuned against a real device rather than against a small desktop window.
- Add optional judgment sounds only after testing them with microphone capture;
  speaker feedback can contaminate pitch detection. Visual feedback is the safe
  default.
- Re-capture the player-guide screenshots and update
  `docs/gameplay_validation.md` as each visible phase lands.

Two defects the Phase 0 pass turned up, both small and both in this phase's
territory (see `docs/gameplay_baseline.md`):

- **The wait-for-note prompt is not localized.** `wait_freeze_overlay.rs`
  builds it with a bare `format!("Play Hole {} {}")`. It escapes `build.rs`'s
  literal check because the literal reaches `Text` through a variable rather
  than directly — so fixing it means a Fluent key *and* deciding whether that
  check should follow a binding one hop, since anything else written this way
  is equally invisible.
- **A chromatic harp renders `? position`.** Diatonic position is meaningless
  on a chromatic harp, so the field should be omitted rather than filled with
  a question mark.

## Implementation boundaries

- Keep `GameplayClock` as the only time authority. Renderers may map its value
  to geometry but may not advance or reinterpret time.
- Keep scoring primitives in `harmonicon-core` and the ECS driver in
  `judge.rs`. UI consumes judgment messages and never decides hits itself.
- Build one shared scored-play HUD used by 2D and 3D. Mode modules supply the
  lane surface and note visuals.
- Derive contextual UI from authored chart data. Do not infer a blues form from
  the song key.
- Land one phase in small commits: data/message contract, shared HUD structure,
  2D integration, 3D integration, tests/docs, and captures are separate tasks.

The recommended order is Phase 0, the conditional/context cleanup from Phase 1,
the shared HUD and highway hierarchy, then richer judgment feedback. Those
changes address the strongest sources of roughness while keeping the stable
audio and scoring foundation intact.
