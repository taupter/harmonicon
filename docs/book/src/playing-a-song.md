# Playing a Song

**Play → Play Song** starts the scored song flow:

1. **Select Mode** — choose [Play 2D](play-2d.md) (a scrolling note
   highway) or [Play 3D](play-3d.md) (a 3D harmonica model you play along
   with). Both modes share the same scoring, timing, and pause menu — it's
   purely a visual choice.
2. **Select Artist**, then **Select Song** — browse the bundled songs (and
   anything you've dropped into `~/Harmonicon/songs/`, see
   [Getting Started](getting-started.md#adding-your-own-content)).
3. A **3-2-1 countdown** plays, showing the song title, key, and which
   physical harmonica to grab, then the chart starts scrolling and the
   backing track plays.

![Mode select screen](images/mode-select.png)

## Scoring

As notes reach the hit line, Harmonicon compares the pitch it hears against
what the chart expects, at that instant:

- **Perfect** / **Good** hits, based on how close your timing was to the
  note's onset.
- **Miss**, if the window passes with nothing (or the wrong pitch) played.
- Longer notes reward **holding** the correct pitch for their full
  duration, not just landing the onset.
- Special techniques — **bends**, **vibrato**, **wah**, **overblow/
  overdraw**, and (chromatic only) **slides** — are validated on their own
  terms, not just "was some pitch playing": a bend note checks you actually
  bent to the target pitch, a vibrato/wah note checks the oscillation rate
  you played matches what the chart asks for.
- **Chords and octave-split notes** only score when every note in the
  group sounds *together* — playing the same holes correctly but one at a
  time doesn't count.

A combo multiplier builds on consecutive hits and resets on a miss. The
**Results screen** after each song is written as coaching, not just a
scoreboard:

- **Accuracy leads, with one observation under it** — the single most
  useful thing the run showed: a technique that trailed your plain notes,
  too many notes that never sounded, hits that consistently came in late
  or early, or attacks with a neighbouring hole leaking. It only says
  something it has evidence for — one attempted bend is never "your bends
  need work" — and says "nothing stands out" when a run is solid.
- **Timing as a distribution**, not an average: an early / on-time / late
  bar with the counts. The one-click **Input lag** adjustment (see
  [Calibrating Input Lag](calibration.md)) appears only when your hits
  actually lean one way, since a wide scatter can average to a number
  without any lag being the cause.
- **By technique**, ranked by where practice would pay off most, always
  with the sample counts alongside.
- **Practice missed section** loops the two bars where you missed the
  most, using the same A–B loop the pause menu offers, and starts you
  there rather than from the top of the song. Leave it with **Esc** →
  **Quit Song**, or clear the loop from the pause menu to carry on
  through the rest.

For a lesson, the pass/fail verdict and how far you got toward its goal
sit above all of that.

![Results screen](images/results-screen.png)

## Pausing and quitting

Press **Esc**, or click the **⏸** button in the bottom-right corner, to
pause mid-song — the on-screen button works the same as Esc, no keyboard
required. The pause menu is two columns: **Resume** / **Restart** / **Quit
Song** on the left, every practice aid on the right, so a slip of the mouse
over one can't misclick the other. The practice aids:

- **Wait for Note** — freezes the highway and music the instant an unhit
  note reaches the hit line, and holds there until you play it — useful
  for slowing down a hard passage without losing your place. There's no
  way to "miss" a frozen note; it just waits.
- **Practice Speed** — a slider from 50% to 100% that slows the highway and
  metronome without pitch-shifting the audio; it mutes instead below 100%,
  so you never hear a chipmunked backing track.
- **Adaptive Difficulty** — off by default (turn it on in **Options**,
  under Audio); once on, a song's notes unlock gradually as you clear each
  phrase cleanly, instead of throwing the full chart at you immediately.
  This is one setting shared by every song, not something you pick per
  song. To override a specific phrase, click its rectangle on the
  song-progress bar's bottom strip (it highlights gold once selected) and
  drag the **Learned** slider that appears below — or flip the pause
  menu's own toggle to switch it off/on immediately, mid-song (this also
  updates the Options-menu setting).
- **A–B Looping** — drag on the song-progress bar at the top of the screen
  to mark a section and loop it, for drilling one phrase repeatedly.
  **Clear Loop** removes it.

See the [Controls Reference](controls.md) for every in-game keybinding.
