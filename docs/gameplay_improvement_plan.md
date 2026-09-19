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

## Phase 2: report what happened at each note — done

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

The in-play presentation is done too (`gameplay::note_feedback`, shared by
2D and 3D so the modes can't drift on what a hit looks like):

- **The judged note animates.** Its head pops to 1.35× and eases back over
  0.22 s on a hit, or shrinks to 0.72× and stays there on a miss, and its
  tab label becomes ✓ / ✗ — the transition is observed once through a
  `JudgedState` component rather than re-derived per frame, which is also
  what makes an A–B loop clean: the loop clears `hit`/`missed`, the state
  reads the change back to pending, and the head is restored. The shrink
  and stamp are the non-colour hit/miss cue Phase 5 asked for on the note
  head itself. (The stamp exposed that `dialogs::font_fallback` ran in
  `Update`, unordered against its writers, so any label that *became* an
  icon flashed one frame of tofu; it now runs in `PostUpdate` ahead of UI
  text measurement.)
- **A hold shows live state, not a fill.** The tail scrolls through the hit
  line time-accurately, so progress is already the line sweeping up it; the
  part still above the line is what the player sees, and it now shows
  whether the expected pitch is sounding this frame (gold / dim grey), how
  much of the hold so far was credited (paler gold for time lost), and the
  sustained technique's live status — `judge::live_technique_status`, from
  the same samples the end-of-hold verdict uses, with "not measurable yet"
  distinct from "heard at the wrong rate" — as a shimmer when confirmed or
  a flattened gold while unheard. All of it rides a fourth `hold` uniform on
  the existing tail materials.
- **Labels stay put.** The judgment readout is one fixed-position line (plus
  the wrong-pitch detail) that new judgments replace rather than stack, so
  dense passages never produce a wall of labels; nothing per-note floats.

A dev-only autoplayer (`gameplay::autoplay`, `brpctl.py autoplay on`) came
out of verifying this: nothing a hit shows was capturable without a mic.

Acceptance: pure/headless tests assert the emitted feedback for perfect,
early/late good, no-attack miss, wrong-pitch miss, incomplete chord, and failed
sustained technique *(done — `gameplay::tests::score_notes_blames_*` and
siblings)*. Live validation confirms that feedback appears on the same visual
beat as the judged note.

## Phase 3: make practice controls part of the loop

- *(Done)* Wait-for-note is a coaching card at the hit line
  (`wait_freeze_overlay`): "Play 8↓" plus a live "hearing 4↑ / listening…"
  line, the heard pitch resolved through the judge's own `heard_tab` so it
  never names a hole the scorer wouldn't. The success transition is the
  judgment readout itself — PERFECT/GOOD fires at the instant the freeze
  lifts, at the same height — so no second flash was added.
- *(Done)* The A–B range on the progress strip has solid A and B handles at
  its ends, children of the range marker so they ride along with it; the
  wash alone had no ends and did not read as *from here to here*. The badge
  under the title carries the seconds. Range editing stays pause-only, as
  before. The selected phrase already had a gold border on its progress-bar
  rectangle while paused.
- *(Done)* The pause menu is three cards — session actions, playback aids,
  phrase practice — on an opaque surface. Phrase controls are hidden in Jam
  Session, the one mode with neither phrases nor adaptive difficulty.
- *(Done)* `practice_badges` names whichever aids are on — "70% speed ·
  music off", "waiting for each note", "loop 12s–20s" — under the title in
  the shared HUD panel. Only active aids show; a row of "off" badges is
  furniture.
- Preserve the existing immediate resource updates and sink-safe rewind path;
  this is a presentation and navigation change around working mechanics.

Acceptance: a player can select a phrase, slow it down, loop it, enable waiting,
resume, and understand every active aid from the live screen. Restart and quit
remain visually separated from practice adjustments.

## Phase 4: turn results into coaching — done

Everything the screen says comes from `gameplay::coaching`, pure functions
over `SongStats`; `results.rs` only lays them out.

- Accuracy leads, with **one observation** under it, chosen in a fixed
  order — a technique trailing plain notes by a margin, a high miss rate,
  a lopsided timing lean, leaky attacks — or "nothing stands out". Every
  branch has a minimum sample size (`MIN_TECHNIQUE_SAMPLES`, `MIN_NOTES`,
  `MIN_TIMING_SAMPLES`), so one attempted bend is never advice; fewer than
  `MIN_NOTES` notes says nothing at all. Grade and score follow on one
  line. The `Hits` row is gone.
- Timing is a **histogram** (`SongStats::timing`, eleven 20 ms buckets
  recorded beside `offset_sum`), shown as an early / on-time / late bar
  with the mean as a caption. The Input-lag button appears only when
  `latency_suggestion` finds the distribution lopsided (≥ 60% of hits on
  the mean's side, ≥ 8 hits, |mean| ≥ 5 ms) — a symmetric scatter with a
  nonzero mean earns no button.
- Technique rows are ranked by misses, then accuracy (`ranked_techniques`),
  sample counts kept on every row.
- **Practice missed section** picks the two-bar window holding the most
  missed notes (`missed_range`, one beat of lead-in, earliest window on a
  tie) and enters the *existing* A–B loop through `PracticeRequest`:
  `setup_scoring_config` copies it into `LoopConfig` and
  `start_at_practice_range` jumps the clock — and seeks the sink — once
  the music sink exists, with the skipped prefix resolved silently first so
  it doesn't tally as misses. This exposed that a loop active at song start
  used to make `SongEnd` infinite, which hid the playhead and loop marker
  and meant clearing the loop could never reach Results; `SongEnd` is now
  always the chart's real end and `detect_song_end` waits on `LoopConfig`.
- A lesson's verdict, its "Goal: …" line (the lesson reader's own wording)
  and "This run: N%" sit above the accuracy.

Acceptance met: `gameplay::coaching::tests::*` cover observation selection
and its floors, the histogram buckets and split, the latency rule, ranking,
lesson progress and missed-range selection; `gameplay::tests::
practice_start_*`/`skip_notes_before_*`/`an_active_loop_holds_off_*` cover
the practice entry. `docs/gameplay_validation.md` has the live checks.

## Phase 5: polish and accessibility

- *(Done, in Phase 2's tail)* Hit state on the note head is no longer
  colour-only: a hit pops and stamps ✓, a miss shrinks and stays shrunk with
  ✗ (`gameplay::note_feedback`). Breath direction was already covered by
  the note-head label (see Phase 1).
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

- **`build.rs`'s literal check doesn't follow a binding.** The wait-for-note
  prompt was a bare `format!("Play Hole {} {}")` reaching `Text` through a
  variable, and the check never saw it (it is localized now, as part of the
  Phase 3 card). Anything else written that way is equally invisible; worth
  deciding whether the scan should follow a `let` one hop.
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
