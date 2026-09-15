# Song Editor Audit and Handoff Plan

## Goal

Make `harmonicon-editor` a safe and practical authoring tool for blues
harmonica, jazz, general music theory, songs, and lessons. Loading and saving an
existing `.harpchart` must never silently discard musical meaning.

## Current state

The work below is committed on `main`, and the working tree was clean after the
last commit.

| Commit | Result |
| --- | --- |
| `380fc07` | Refuse imports that the editor cannot round-trip safely. |
| `96c4779` | Report MIDI notes and chords that need playability review. |
| `78e474b` | Support 16-hole chromatic charts. |
| `cdc423c` | Support Paddy Richter and natural-minor diatonics. |
| `d2ce1e8` | Score MIDI key suggestions against the selected harp layout. |
| `d2d6cde` | Document the pre-existing oversized lesson reader in the design budget. |
| `e5d7fe6` | Give chord symbols their own chart field. |
| `fc54d0b` | Author section and chord markers from Details. |
| `d60e846` | Author and preserve call-and-response phrases. |
| `e4abf26` | Author and preserve tongue-block split play mode. |
| `2c7df0e` | Author and preserve phrase-level groove guidance. |
| `de67b0e` | Author and preserve per-note vibrato/wah intensity; split selected metadata out of `state.rs`. |
| `411fc0e` | Support country-tuned diatonics, including raised draw 5. |
| `18be7af` | Preserve metadata, difficulty/feel, scoring, and loop settings that lack editor controls. |
| `80f0090` | Read a chart's meter in one place; the metronome clicks the meter's own beat. |
| `ffdee1c` | Preserve a chart's custom harmonica layout via `EditorState::effective_harp`. |
| `23d131b` | Carry phrase annotations and expression intensities through every bulk edit (`metadata_sync`). |
| Phrase annotation lane | Show section, chord, groove, call-response, and split markers above the chart. |

User changes made during the same effort are also part of the current base:
snap-aware grid backgrounds, reachable external resize grips, a meter picker,
and tick-based odd-meter ruler layout. Preserve those designs.

The last complete validation was:

```text
cargo test -p harmonicon-editor                         307 passed
cargo test --test physical_design                       3 passed
cargo clippy --workspace --all-targets -- -D warnings   passed
cargo fmt --all -- --check                              passed
```

Use `RUSTC_WRAPPER=` on cargo commands if the configured wrapper interferes.
The physical-design suite contains
`no_file_exceeds_the_line_budget_unless_allowlisted`; run it for every slice.
Do not casually add new allowlist entries. `state.rs` was reduced below the
1,000-line limit by moving selected-note behavior to `selected_metadata.rs`.

## Remaining work, in priority order

### 1. Support time-signature maps — complete

`timing.time_signature_map` is represented and editable end to end:

1. Add an editor meter-map representation with a tick-zero effective meter.
2. Load/save maps with resolution conversion.
3. Make ruler bar numbering and boundaries integrate meter segments without
   resetting or drifting at changes.
4. Make metronome count-in, notation splitting, waveform/grid alignment, and
   any bar-based overlays ask the meter map at the relevant tick.
5. Add a timeline tool or marker editor for inserting/removing meter changes.
6. Import MIDI meter changes rather than retaining only one meter.

The live editor metronome and Record count-in now use the meter segment active
at the playhead. Notation splits sustained notes at meter-map bar boundaries
and displays the active meter. MIDI import now preserves the full meter map.

Cover 4/4 → 3/4, 6/8 → 7/8, changes away from a bar line, and round-trip at a
non-480 source resolution. The unsupported-feature rejection has been removed.

### 2. Add controls for preserved song settings — complete

Commit `18be7af` prevents data loss by retaining settings as JSON, but authors
still cannot edit them. Add typed state and Details controls for:

- Difficulty — typed state and a Details cycle control are implemented
- Straight/shuffle feel — a separate Details cycle control is implemented;
  `default` omits the optional field and does not alter placement snap
- Source, license, and description — typed Details text fields are implemented
- Perfect/good/miss scoring windows and combo behavior — typed Details fields
  are implemented; style bonuses remain preserved
- Loop type, repeat, and range — typed Details controls are implemented and
  indices clamp to the current phrase range on save

When loop indices refer to phrase ordering, note insertion/deletion must keep
the loop meaningful or show a clear validation error. Replace preserved JSON
with typed fields incrementally, maintaining old-file round trips throughout.

### 3. Decide valid modifier combinations — complete

The editor represents one pitch technique plus one expression technique. It
supports combinations such as bend + vibrato and rejects multiple pitch
modifiers or multiple expression modifiers in one event.

This remains the deliberate model: pitch techniques determine the target pitch
and tab label, expressions determine sustained rendering, and scoring credits
both categories. Multiple pitch techniques have no unambiguous target; multiple
expressions cannot be reproduced by the single-expression synth. Load errors
name the conflicting category and phrase/event location.

### 4. Instrument extensibility

The current schema limits diatonics to ten holes and four named profiles, and
chromatics to 10, 12, or 16 holes. Tests pin those boundaries before editor
loading; duplicate post-schema checks were removed because no schema-valid
chart could reach them.
Custom layouts are retained (`ffdee1c`), and **bend depth is now derived from
the reeds** rather than a Richter table — which had disagreed with the
alternate tunings on nine holes, forbidding country tuning's hole-5 bend and
permitting a three-semitone bend on Paddy Richter's hole 3. `technique_fits`
takes the harp; `overblow_ok`/`overdraw_ok` stay by hole number as an
instrument convention, not physics.

What a generic layout-backed `HarmonicaKind` would still need: a hole count
read from the layout rather than the kind (65 `HarmonicaKind::` match sites
across ten editor files, most of them for that), a save that writes no
`bending_profile`, and a decision on over-technique availability for reed
pairs Richter doesn't have (draw-above-blow on holes 7+, or the reverse on
1–6). Not started; the named-preset model plus retained custom layouts covers
every bundled and importable chart today.

### 5. Final usability and integration pass

- Run the editor at desktop and short landscape/mobile dimensions — done over
  BRP at 1280×720 and 960×360 logical: Details scrolls and every field is
  reachable at both; the Chart tab's grid (~446 px for ten holes with the
  annotation lane) exceeds the short viewport, reachable only by two-finger
  touch pan, which is the height-aware `CompactLayout` item PLAN.md defers to
  post-1.0 and predates this audit.
- Verify Details remains scrollable after the added fields and checkboxes —
  verified, see above. `Description` now uses a four-line word-wrapped editor;
  Enter inserts a newline and losing focus commits the value.
- Every bundled `.harpchart` is now covered by automated load/resave/schema and
  semantic comparison. The comparison checks events, modifiers, annotations,
  timing maps, song/harmonica settings, scoring, loops, and metadata while
  allowing grid quantization, generated phrase IDs, derived pitch labels, and
  explicit serialization of schema defaults. It exposed and fixed the editor's
  conflation of `metadata.author` with `song.artist` (`73213a4`). Synthetic unit
  coverage handles alternate diatonic tunings, 12/16-hole chromatic,
  meter/tempo maps, split, call-response, and custom expression intensity.
- Update `docs/book/src/song-editor.md`,
  `contributing/src/song-editor-architecture.md`, and
  `crates/harmonicon-editor/CLAUDE.md` when an invariant becomes load-bearing.

## Workflow for the next chat

1. Read the root and `crates/harmonicon-editor/CLAUDE.md` instructions.
2. Run `git status --short` and inspect recent commits; preserve any new user
   work in the song editor.
3. Items 1–3 are complete; what remains is the layout-backed instrument (4)
   and the rest of the integration pass (5).
4. Keep each vertical slice small, fully tested, documented, and committed.
5. Before each commit run:

```bash
cargo fmt --all -- --check
RUSTC_WRAPPER= cargo test -p harmonicon-editor
RUSTC_WRAPPER= cargo test --test physical_design
RUSTC_WRAPPER= cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

The user explicitly requested commits and repeatedly asked to continue, so
commit each completed, green slice rather than leaving accumulated changes in
the working tree.
