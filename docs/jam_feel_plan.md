# Jam Session: listening acceptance

The six implementation phases are complete. This document tracks the
remaining listening review; `PLAN.md` is the open-work index.

## Product boundary

Jam Session is where a player goes to make music, stay in time, try an idea,
lose it, and find the groove again. It is not a lesson with the score hidden.

The bending trainer owns isolated control of bends. Lessons own explanation,
prescribed exercises, pass criteria, and progress. Jam Session may show the
musical context, offer an optional phrase, and let the backing react to the
player, but it must not mark notes right or wrong, issue a grade, require a
pattern, or gate anything behind success.

This distinction also applies inside the crate. `jam::lesson`, the position
cycle, and `ImprovStats` remain adapters used when the lessons engine chooses
Jam Session as its performance surface. They should not shape the ordinary jam
experience or become its product direction.

## Deferred listening acceptance

Deferred on 2026-09-22 because audio review was not practical at the time. No
implementation work is blocked by it, and it must not be replaced with
waveform-only approval.

Resume with:

```sh
cargo run -p harmonicon-jam --example listening_matrix
```

This regenerates the fixed-seed Blues/Jazz/Reggae × 70/100/140 BPM files in
`target/jam-listening/`. Then complete the Jam Session rows in
`docs/gameplay_validation.md`, including live checks for the four-chorus arc,
Low/High energy, 1st/2nd-position register space, call ducking and timbre,
adaptive thinning/answers, stem mutes, and the scheduled full-band ending.
The review outcome should either mark those rows complete or name concrete mix
changes; broad impressions are not actionable enough to tune against.

This listening pass is the only remaining acceptance work in this plan.

## Deliberately out of scope

- bend drills, technique correction, note targets, accuracy, pass/fail, streaks,
  curriculum progression, and post-session grading;
- claiming stylistic authenticity beyond a 12-bar practice setting;
- unrestricted live tempo/key changes while audio is sounding;
- self-monitoring the microphone through speakers, which risks feedback and
  belongs to audio-device design rather than accompaniment;
- recorded backing packs until the generated rhythm section establishes the
  interaction and mixer model they would plug into.
