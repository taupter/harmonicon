# Jam Session: feel-first improvement plan

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

## What exists today

The crate has a sound foundation:

- generated 12-bar backing in any key, tempo, position, scale, progression,
  and one of five genres;
- a shared gameplay clock, chord grid, metronome, looping, countdown, pause,
  and song-progress infrastructure;
- live chord-tone/in-scale/out-of-scale coloring on the player's actual harp;
- optional, unscored call-and-response;
- a genre rhythm pulse derived from the same pattern as the generated audio;
- real-song backing, including independently mutable MIDI tracks;
- deterministic pure functions and focused tests around timing and harmony.

At the start of this pass, the musical experience was weaker than the feature
list suggested:

1. The generated band is one sine-based bass voice. `Genre` changes an
   eight-slot bass pattern and straight/shuffle feel, but there is no drummer,
   chordal instrument, backbeat, turnaround, fill, accent shape, or ensemble
   dynamic. Five labels therefore produce variations of a practice pulse more
   than five convincing feels.
2. Every bar within a genre uses the same pattern. Eight rendered choruses have
   no arc, and repeating the buffer repeats the same performance exactly.
3. Generated Jam is described as endless, while `JamLoop` defaults off. The
   backing can simply stop after its rendered choruses unless the player finds
   and enables Loop.
4. The screen gives equal weight to title, harp hint, loop, call-and-response,
   12-bar grid, metronome, rhythm guide, spectrogram, bend diagram, hole map,
   position compass, progress bar, and optional MIDI controls. Those are useful
   diagnostics, but together they pull attention away from listening and
   breathing.
5. Call-and-response chooses four chord tones and puts one on every beat. It is
   harmonically safe but rarely sounds like a phrase a harmonica player would
   want to answer.
6. The backing never notices the player. Silence, a held note, a dense run, and
   a strong two-bar phrase all receive the exact same accompaniment.

## Desired experience

A generated jam should reach the downbeat quickly, establish an unmistakable
groove through sound alone, leave space for harmonica, build gently across
choruses, signal the turnaround, and either continue cleanly or end like a
band. The player should be able to look away from the screen for a full chorus
and remain oriented.

Visuals should answer only three immediate questions: where are we in the
form, what chord is sounding next, and what did the microphone hear? Theory
references and analysis remain available on demand.

The mode succeeds when players keep playing after a mistake because the groove
still carries them. Session length, use of complete choruses, and voluntary
restarts are more relevant signals than note accuracy.

## Implementation order

### 1. Make the generated session continuous and finishable

**Implemented.** Generated sessions continue independently of the finite-song
Loop preference, show chorus/bar position, and can queue a tonic ending at the
next chorus boundary. The later rhythm-section phase can replace that simple
tonic punctuation with a full-band ending.

Fix the basic performance contract before adding instruments.

- Give generated jams their own playback policy: continue by default, while a
  picked finite song retains the existing opt-in Loop behavior. Do not overload
  one persistent `JamLoop` preference with two different expectations.
- Replace the binary Loop control for generated sessions with `Keep playing`
  and `End after this chorus`. The latter schedules a musical ending at the
  next bar 12 rather than cutting a sink immediately.
- Show a small `Chorus N · Bar M` position and make bar 11/12 visibly signal
  the turnaround. Keep the full 12-bar grid as an optional expanded view.
- Ensure restart begins with the existing count-in and resets chorus state,
  call-and-response state, and any scheduled ending together.

Exit criteria: a generated jam never falls silent unexpectedly; a player can
request an ending without taking a hand off the harmonica at an exact instant;
the audio and form display agree across repeated choruses.

### 2. Turn the generator into a small rhythm section

**In progress.** Generated backing now consists of sample-aligned Bass, Drums,
and Comping stems using the generalized backing-stem playback/mute path. The
first deterministic genre patterns and a shared `GrooveArrangement` event
model are in place. Restrained fills now mark bars 4, 8, and 11, while bar 12
clears comping space for a chromatic bass-and-drum turnaround. Drum and comping
voices now change with genre, including a longer jazz ride/organ texture, a
short reggae skank, a firmer rock backbeat, and a lighter country pulse. A
balance listening pass and genre-credibility review remain.

Model accompaniment as synchronized stems rather than one mixed bass buffer.
Reuse the independent-sink and mute machinery already proven by MIDI backing,
generalized as `BackingStemAudio`/`JamStemMute` so generated and MIDI stems
share one playback path.

Start with three roles:

- **Drums:** kick, snare/rim, hi-hat or ride, with velocity accents and a short
  fill vocabulary.
- **Bass:** retain the current register and speaker-aware harmonics, but add
  approach notes, rests, pickup notes, and bar-dependent patterns.
- **Comping:** a quiet guitar/organ/piano-like chord voice that stays out of the
  harmonica register and supports chord quality, especially minor and jazz
  changes.

Create a `GrooveArrangement` data model containing meter, subdivision feel,
per-role events, accent strength, and fill/turnaround variants. The audio
renderers and any visual pulse must consume this model; the UI must not infer a
groove from a second set of constants.

Each genre needs a musically distinct arrangement within the existing 12-bar
scope:

| Feel | Rhythm-section identity |
|---|---|
| Blues | shuffle hats, backbeat, walking/box bass, V-bar turnaround |
| Jazz | ride pattern, light 2-and-4, walking quarters, sparse shell voicings |
| Rock | straight eighths, firm backbeat, root/fifth bass, power-chord pushes |
| Reggae | one-drop or steppers drums, offbeat skank, syncopated bass space |
| Country | train/boom-chick pulse, alternating bass, light chord chops |

Keep generation deterministic from a stored seed. Tests should assert event
timing, form length, chord membership, stem synchronization, and bounded peak
level. Listening review is required for balance and genre credibility; waveform
non-silence tests cannot establish feel.

Exit criteria: each genre is recognizable with the screen hidden; muting any
one role leaves the other roles synchronized; the harmonica has audible space.

### 3. Give each four-chorus arc a shape

**Implemented.** The generated buffer now follows a deterministic four-chorus
dynamics arc: sparse comping and softer drums, a comping lift, the fullest
band texture, then a relaxed chorus before repeating. The Low, Medium, and
High Band energy control changes accompaniment density and dynamics without
changing the form. Each role also gets deterministic microtiming and velocity
variation within its rhythmic slots; quarter-note downbeats and every slot's
length stay fixed, so the performance loosens without accumulating drift.

Arrange at the chorus level instead of cloning one 12-bar block.

- Chorus 1 establishes the pocket with sparse comping.
- Chorus 2 adds a small lift or answer at phrase boundaries.
- Chorus 3 reaches the fullest normal texture.
- Chorus 4 either relaxes into another four-chorus arc or plays the scheduled
  ending.
- Bars 4, 8, 11, and 12 may select restrained fills; fills must never obscure
  the next downbeat or occupy the whole harmonica register.
- Add small deterministic timing and velocity variation per role. Preserve the
  shared bar clock; humanization may move attacks within a safe window but may
  not accumulate drift.

Expose one musical control, `Band energy` (Low, Medium, High), which changes
density and dynamics without changing tempo or progression. Avoid a panel of
synthesis parameters.

Exit criteria: consecutive choruses are related but not identical, the player
can hear the approach to bar 1, and every energy level preserves the groove.

### 4. Make listening the default interface

**Implemented.** The default stage centers a large current-to-next chord
readout, chorus/bar position, a compact twelve-cell form strip and a
one-line detected hole/breath indicator tinted by fit. A persistent Guides
toggle collapses the full grid, metronome, rhythm pulse, spectrogram, bend
diagram, hole map, and position compass together, expanding the primary
stage into the freed space. "Eyes off" is that toggle: with guides hidden,
what remains is form, transport and the one restrained indicator, so a
separate mode would only differ by hiding a single line.

Split the current display into a calm default stage and optional guides.

The default stage shows:

- current chord, next chord, chorus/bar, and a compact form strip;
- a restrained live indication of the detected hole/direction;
- transport, ending, and band-energy controls;
- a compact stem mixer when generated or MIDI stems exist.

An expandable `Guides` drawer holds the full 12-bar grid, bend diagram, scale
coloring, position compass, rhythm pulse, metronome controls, and spectrogram.
Remember guide visibility between jams. Call-and-response remains a separate
creative toggle because it changes the music, not merely the display.

Color feedback in ordinary jams should describe rather than judge. A played
note may glow with its harmonic relationship, but the interface must not flash
failure, count misses, show percentages, or retain an error history. Provide an
`Eyes off` option that hides every guide except form and transport.

Exit criteria: the default screen has one clear visual center; all existing
diagnostic tools remain reachable; an entire chorus is playable without
looking at a theory diagram.

### 5. Make call-and-response sound like phrasing

**Implemented.** `jam::call_response::phrase` composes each call from a
rhythm cell per bar, a two-to-four-note motif and a repeat/up/down answer,
constrained to the player's harp and resolved onto a chord tone with two
beats of air before "Your turn"; a `Phrasing` cycle button picks Sparse,
Conversational or Busy; the backing ducks to 60 % while the call sounds.
Still to review by ear: whether the harmonica-synth call sits well against
each genre's rhythm section and whether the duck depth is right on
speakers.

Keep it optional, unscored, and forgiving. Replace four random quarter-note
chord tones with a phrase generator built from small musical decisions:

- rhythm cells containing rests, pickups, syncopation, held notes, and repeated
  notes;
- two-, three-, and four-note motifs with a limited range and playable breath
  direction changes;
- contour choices (repeat, answer upward, answer downward) and occasional blue
  notes or scale neighbors resolving to chord tones;
- phrase endings that leave audible space before `Your turn`;
- difficulty expressed only as musical density (`Sparse`, `Conversational`,
  `Busy`), never as a level or score.

The generated call should use a distinct, band-compatible harmonica timbre and
duck the backing slightly while it speaks. Ghost holes are optional; the audio
must be sufficient on its own. During the response the app listens and lights
what it hears, but does not compare the response with the call.

Exit criteria: calls have breath and contour, no call requires an impossible
note on the selected harp, and a player can answer with a variation without the
UI implying failure.

### 6. Let the band listen without grading

**Implemented.** `jam::band` logs coarse per-beat activity (attack count,
presence), thins the comping for the phrase after a dense one, holds the
pocket through silence, and answers a released phrase at beat 3 of a
four-bar phrase's last bar with a drum fill or chord push, at most once per
eight bars; an `Adaptive band` toggle switches it off. Decisions are pure
functions of the beat log and the tests replay a recorded stream. Still to
review by ear: the thinning depth and whether the answers sit inside each
genre's groove.

Once the rhythm section and phrase boundaries are stable, add restrained
reactivity driven by coarse musical activity rather than pitch correctness.

- Track phrase onset, release, density, and sustained silence over musical
  windows. Do not classify these as good or bad.
- After a dense phrase, let the next backing phrase simplify. After a long
  silence, keep the pocket rather than filling every gap immediately.
- Permit a short drum or chordal answer only in known phrase-end windows.
- Smooth all state changes at bar boundaries and cap how often the band may
  react, so microphone noise cannot make the arrangement twitch.
- Add an `Adaptive band` toggle and make its current behavior legible through
  sound; no coaching text is needed.

Exit criteria: reactions are musically timed, deterministic under a recorded
activity stream, and never change harmony, tempo, or volume abruptly.

## Architecture and verification notes

- Keep musical decisions as pure data transforms: form + genre + seed + energy
  produce an arrangement; arrangement renderers produce synchronized PCM.
- Keep the existing clock as the authority for bar/chorus state. Audio, form
  UI, endings, fills, and call-and-response must share that state.
- Generalize stem playback below `jam` only if both generated and MIDI backing
  need it. Preserve the current dependency direction between `jam`, `gameplay`,
  and `song`.
- Do not put ordinary-jam behavior behind `LessonContext`, `PassCriteria`, or
  `ImprovStats`. Lesson adapters may observe the jam; the jam must not depend on
  their evaluation.
- Unit tests cover form, timing, playable range, deterministic variation,
  synchronization, transitions, and state reset. They cannot approve groove,
  mix, timbre, or whether a fill leaves room. Each audio phase needs a short
  listening matrix across at least Blues/Jazz/Reggae, 70/100/140 BPM, and first/
  second position on laptop speakers and headphones.

## Deliberately out of scope

- bend drills, technique correction, note targets, accuracy, pass/fail, streaks,
  curriculum progression, and post-session grading;
- claiming stylistic authenticity beyond a 12-bar practice setting;
- unrestricted live tempo/key changes while audio is sounding;
- self-monitoring the microphone through speakers, which risks feedback and
  belongs to audio-device design rather than accompaniment;
- recorded backing packs until the generated rhythm section establishes the
  interaction and mixer model they would plug into.
