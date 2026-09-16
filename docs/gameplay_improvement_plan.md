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

## Phase 0: establish a visual baseline

- Capture Play 2D, Play 3D, pause, wait-for-note, and results at the supported
  wide and compact sizes using the existing BRP capture workflow.
- Use three fixtures: a simple single-note song, a technique-heavy song, and a
  chromatic/chord song. Include one run with no detected input and one with a
  scripted mixture of hits and misses.
- Add layout assertions only for pure decisions such as which contextual panels
  are visible. Rendering quality remains screenshot/manual validation.

This gives every later phase a before/after comparison and catches the current
compact-layout failure modes before rearranging the screen.

## Phase 1: make the highway the visual focus

- Move the score, combo, and transient judgment into a compact overlay close to
  the hit line. Keep the song title in a small persistent header; show the full
  key, harp, author, and description during countdown and pause instead of
  spending the whole performance on them.
- Give the 2D highway most of the available width. Add beat and downbeat guides
  derived from `GameplayClock`, the tempo map, and `bars::chart_meter`; do not
  introduce a second timing calculation in the renderer.
- Strengthen the hit line and align hole numbers with it. Use shape or labels in
  addition to blow/draw color so the colorblind palette is not the only cue.
- Keep technique symbols close to the notes that use them; the legend is
  supporting help, not permanent primary content. (Restricting the legend to
  techniques the chart actually uses is done.)
- Apply the same information hierarchy to Play 3D. Preserve the different lane
  rendering, but share the surrounding HUD composition so the two modes do not
  drift again.

Acceptance: at 1280×720 and the compact breakpoint, the next notes, hit line,
expected hole/direction, score, and pause control are all legible without
overlap. A non-blues chart shows no invented blues form. *(The invented blues
form is gone; the layout work is open.)*

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

Remaining in-play presentation work:

- briefly show expected and heard hole/tab information for a wrong-pitch miss;
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
timing histogram buckets, and missed-range selection. The result remains usable
at compact height through scrolling or a responsive two-column layout.

## Phase 5: polish and accessibility

- Finish the colorblind-safe note palette work already listed in `PLAN.md`, with
  shapes/marks for breath direction and hit state so color is never the sole
  signal.
- Add reduced-motion and feedback-intensity settings before introducing camera
  shake or large pulses. Visual motion must never move the hit target.
- Audit text contrast, focus order, touch target size, localization expansion,
  and short landscape windows. The shared responsive decision should become
  height-aware rather than adding gameplay-only pixel exceptions.
- Add optional judgment sounds only after testing them with microphone capture;
  speaker feedback can contaminate pitch detection. Visual feedback is the safe
  default.
- Re-capture the player-guide screenshots and update
  `docs/gameplay_validation.md` as each visible phase lands.

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
