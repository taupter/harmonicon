# Scored-play visual baseline

Phase 0 of `gameplay_improvement_plan.md`: what the scored-play screens
actually look like before the HUD work, so every later phase has a
before/after and so the compact failure modes are known rather than
discovered halfway through a rewrite.

## Retaking it

```bash
cargo run --release --features dev          # in one terminal
python3 scripts/brpctl.py --take-all-screenshots
```

Fourteen PNGs land in `target/screenshots/tour/`. Deliberately not committed:
the command regenerates them in a few minutes, and a committed set goes stale
silently — which is the failure mode a baseline exists to prevent. What is
worth keeping is the list of findings below.

Fixtures, chosen so the contextual parts of the HUD differ:

| Fixture | Song | Why |
|---|---|---|
| `simple` | Traditional — Amazing Grace | no modifiers, no chords: the plain case |
| `technique` | Example Artist — One Bourbon, One Scotch, One Beer | bend, vibrato, wah; a blues chart in 2nd position |
| `chromatic` | Beethoven — Für Elise | 12-hole chromatic, slide; the non-blues chart the plan names |

Sizes are 1280×720 (the plan's wide acceptance size) and 800×600 (below
`CompactLayout`'s 900px breakpoint).

## What already works

Confirmed on screen, not just in tests:

- **No invented blues form.** Für Elise and Amazing Grace show no twelve-bar
  grid. The blues chart doesn't either — correct, since form has to be
  authored rather than inferred from a key.
- **The technique legend lists only what the chart uses**: Bend/Vibrato/Wah
  on the blues chart, Slide alone on Für Elise, and no legend at all on
  Amazing Grace.
- **Judgment labels render and localize.** `MISS` and `WRONG NOTE` appear at
  the right moments, and the wrong-pitch caption (`wanted 8↓ · heard 1↓`)
  draws its arrows and separator from the bundled font rather than tofu.

## Findings

### 1. The pause menu is unreadable at both sizes

The practice controls are painted straight over the song-info panel with no
backdrop of their own, so two independent text columns collide: "Wait for
Note: off" lands on top of "Key: C ♩ = 80 3/4", "Adaptive Difficulty: off"
on top of the description paragraph, "Drag on the progress bar above to set a
loop range" across the metronome buttons. Worse at 800×600, but already
broken at the supported wide size.

This is the single largest visible defect in scored play and it is not a
compact-only problem.

### 2. Results is unusable at compact height

At 800×600 the "SONG COMPLETE" heading is clipped off the top and **Retry and
Continue are pushed off the bottom entirely**, with no scrolling — the player
has no visible way off the screen. Phase 4's acceptance criterion already
calls for scrolling or a two-column layout; this is what it is for.

### 3. 2D and 3D have drifted into different HUDs

Same information, different corners. In 2D the song panel is on the right and
the score sits bottom-right; in 3D the panel is top-left and the score is
top-right in an oversized box. The BLOW/DRAW legend is centre-bottom in 2D and
inside the info panel in 3D. 3D has no hole/note map at all. Phase 1's "one
shared scored-play HUD" is the fix; this records how far apart they are first.

### 4. The judgment is nowhere near the hit line

In 2D the hit line sits mid-screen while the score and judgment are in the
bottom-right corner; in 3D the hit line is centre-screen and the judgment is
top-right. Reading the verdict means looking away from the notes — the
specific complaint Phase 1 opens with.

### 5. Song metadata outweighs the highway at 1280×720

The right panel takes roughly 40% of the width for the whole performance:
title, key, harp, a multi-line description and the chart author. The
description alone wraps to four lines at compact width.

Curiously, **compact is closer to what Phase 1 wants than wide is** — below
the breakpoint the info panel and the notation staff are dropped entirely and
the highway takes the full width. The compact layout has already made the
editorial decision the wide layout hasn't.

### 6. The 3D info panel occludes the highway when compact

At 800×600 the 3D song panel keeps its full width and covers the left half of
the lanes, hiding falling notes behind it. It is not resized for compact at
all.

### 7. The wait-for-note prompt is drawn at the note, not at the hit line

"Play Hole 8 ↓" renders at the frozen note's current position, mid-highway,
overlapping the note it describes. Phase 3 wants this at the hit line.

### 8. Two smaller things worth fixing while nearby

- `wait_freeze_overlay.rs` builds that prompt with a bare
  `format!("Play Hole {} {}")` — unlocalized. It escapes `build.rs`'s literal
  check because the literal never directly reaches a `Text` constructor.
- Für Elise's harp line reads `Chromatic · 12 holes · ? position`. A
  chromatic harp has no diatonic position, so the field should be omitted
  rather than rendered as a question mark.
