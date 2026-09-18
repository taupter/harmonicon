# Scored-play visual baseline

Phase 0 of `gameplay_improvement_plan.md`: what the scored-play screens
actually look like before the HUD work, so every later phase has a
before/after and so the layout failure modes are known rather than
discovered halfway through a rewrite.

## Retaking it

```bash
cargo run --release --features dev          # in one terminal
python3 scripts/brpctl.py --take-all-screenshots
```

Seven PNGs land in `target/screenshots/tour/`. Deliberately not committed:
the command regenerates them in a few minutes, and a committed set goes stale
silently — which is the failure mode a baseline exists to prevent. What is
worth keeping is the list of findings below.

Fixtures, chosen so the contextual parts of the HUD differ:

| Fixture | Song | Why |
|---|---|---|
| `simple` | Traditional — Amazing Grace | no modifiers, no chords: the plain case |
| `technique` | Example Artist — One Bourbon, One Scotch, One Beer | bend, vibrato, wah; a blues chart in 2nd position |
| `chromatic` | Beethoven — Für Elise | 12-hole chromatic, slide; the non-blues chart the plan names |

**Full HD (1920×1080) is the supported floor**, and the only size the tour
captures. Even phones ship 1080p panels, so there is no "small desktop" case
to design against; anything narrower is Android-portrait territory, which
nobody has run on hardware yet. A first pass baselined 800×600 as well and
the two findings unique to it — a clipped results screen, a 3D info panel
covering the lanes — were retired as soon as that stopped being a target.
They are the reason this paragraph exists: a baseline at a size nobody runs
manufactures work.

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

### 1. The pause menu was unreadable — fixed

The practice controls were painted straight over the song-info panel with
only a 65%-black wash between them, so two independent text columns collided:
"Wait for Note: off" landed on top of "Key: C ♩ = 80 3/4", "Adaptive
Difficulty: off" on the description paragraph, "Drag on the progress bar
above to set a loop range" across the metronome buttons.

Now three cards on an opaque surface — session actions, playback aids, phrase
practice — and the overlay reserves the song-progress bar's height at the top,
since that bar deliberately paints *above* the pause menu so a loop range can
be dragged while paused. Dimming controls emphasis; only an opaque surface
controls legibility.

### 2. 2D and 3D had drifted into different HUDs — fixed

Same information, different corners: two separate score readouts at different
font sizes in opposite corners, and two separately-built side panels on
opposite sides. Both are now single spawners (`hud::spawn_score_readout`,
`hud_panel::spawn_hud_panel`) that each mode calls.

What stays different follows from the lane surface rather than from drift:
3D has no hole map, prints the blow/draw key inside its panel because it has
no hole strip to print it under, and has no beat guides because its lane is
world-space geometry.

### 3. Song metadata held a whole column for the whole performance — fixed

The 2D right panel spent roughly a third of the width on title, key, harp, a
multi-line description and the chart author, none of which changes during a
run. It is now a title alone (`song_info::spawn_song_header`); the rest shows
during the countdown and in the pause menu, the two moments the player is not
playing. The 2D highway took the reclaimed width, 60% to 74%.

### 4. The wait-for-note prompt was drawn mid-highway — fixed

"Play Hole 8 ↓" rendered at a fixed height up the highway, over whichever
note was scrolling past. It is now a card at the hit line, anchored the same
way the score readout is, with a second line naming what the mic hears.

### 5. One smaller thing worth fixing while nearby

- Für Elise's harp line reads `Chromatic · 12 holes · ? position`. A
  chromatic harp has no diatonic position, so the field should be omitted
  rather than rendered as a question mark.
