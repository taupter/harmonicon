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

### 1. Preserve custom harmonica layouts safely

`load_harpchart` identifies a named harmonica kind but ignores the serialized
`harmonica.layout`. Saving then regenerates the preset layout. A valid custom
reed layout can therefore be silently replaced.

Implement one of these safe models:

- Retain the loaded layout and its instrument identity, and reuse it while the
  key, kind, hole count, and tuning profile remain compatible.
- Better: make the effective editor harp/layout explicit state and transpose or
  replace it only through an intentional instrument change.

The grid, note audition, practice playback, pitch mapping, MIDI key suggestion,
note names, bend limits, and serialization must all consult the same effective
layout. Add a regression test that changes at least one reed from every named
preset, loads the chart, saves it, and compares the layout exactly.

### 2. Keep note- and phrase-attached metadata aligned during edits

Current metadata is stored separately:

- Phrase annotations are keyed by onset tick.
- Expression intensities are keyed by stable note ID.

Audit and fix these operations:

- Moving the only note or a whole same-onset group should move section, chord,
  groove, call, and split metadata with the phrase.
- Copy/paste should copy expression intensity and relevant phrase annotations.
- Erase and delete should remove metadata only when no note remains at its
  anchor.
- Remove-range should shift later tick-keyed annotations by the removed length
  and remove annotations inside the cut.
- Harmonica-kind sanitization, recording punch-in, and undo/redo should not
  leave stale note-ID metadata.

Write behavioral tests for each operation before changing storage shape. A
small `Phrase`/onset model may be cleaner than adding more special cases to
note movement.

### 3. Display phrase information on the chart

Section, chord, groove, call-response, and split values can be edited in
Details but are not visibly reviewable across the chart. Add compact markers to
the ruler or a dedicated annotation lane. Requirements:

- Markers remain aligned during horizontal scrolling and zooming.
- Chord symbols and section boundaries are readable without selecting notes.
- Call/split markers have distinguishable icons or short labels and tooltips.
- Dense annotations degrade gracefully instead of covering resize grips or
  notes.
- Reuse the tick-keyed annotation data; do not duplicate content in ECS
  components.

### 4. Support time-signature maps

`timing.time_signature_map` is the main remaining valid chart feature rejected
by `unsupported_chart_features`. This is a larger vertical slice:

1. Add an editor meter-map representation with a tick-zero effective meter.
2. Load/save maps with resolution conversion.
3. Make ruler bar numbering and boundaries integrate meter segments without
   resetting or drifting at changes.
4. Make metronome count-in, notation splitting, waveform/grid alignment, and
   any bar-based overlays ask the meter map at the relevant tick.
5. Add a timeline tool or marker editor for inserting/removing meter changes.
6. Import MIDI meter changes rather than retaining only one meter.

Cover 4/4 → 3/4, 6/8 → 7/8, changes away from a bar line, and round-trip at a
non-480 source resolution. Remove the unsupported rejection only after every
consumer preserves the map.

### 5. Add controls for preserved song settings

Commit `18be7af` prevents data loss by retaining settings as JSON, but authors
still cannot edit them. Add typed state and Details controls for:

- Difficulty
- Straight/shuffle feel, kept conceptually separate from placement snap
- Source, license, and description
- Perfect/good/miss scoring windows and combo behavior
- Loop type, repeat, and range

When loop indices refer to phrase ordering, note insertion/deletion must keep
the loop meaningful or show a clear validation error. Replace preserved JSON
with typed fields incrementally, maintaining old-file round trips throughout.

### 6. Decide valid modifier combinations

The editor represents one pitch technique plus one expression technique. It
already supports combinations such as bend + vibrato, but rejects multiple
pitch modifiers or multiple expression modifiers in one event. Review the
schema and gameplay semantics before broadening this:

- Keep musically contradictory combinations invalid with a clear diagnostic.
- Model any meaningful combinations explicitly rather than preserving an
  opaque modifier list that the grid cannot edit.
- Ensure playback, scoring, labels, and serialization agree.

### 7. Instrument extensibility

Unknown future diatonic profiles and chromatics above 16 holes remain rejected.
After custom layouts are safe, consider a generic layout-backed instrument
variant. Avoid adding named variants without complete reed, bend, playback,
import, and save/load behavior.

### 8. Final usability and integration pass

- Run the editor manually at desktop and short landscape/mobile dimensions.
- Verify Details remains scrollable after the added fields and checkboxes.
- Load and resave representative bundled charts: Richter, country tuned,
  natural minor, Paddy Richter, 12/16-hole chromatic, split, call-response,
  tempo-map, and custom expression-intensity lessons.
- Compare semantic JSON before/after, allowing only intentional normalization
  such as formatting and generated phrase IDs.
- Update `docs/book/src/song-editor.md`,
  `contributing/src/song-editor-architecture.md`, and
  `crates/harmonicon-editor/CLAUDE.md` when an invariant becomes load-bearing.

## Workflow for the next chat

1. Read the root and `crates/harmonicon-editor/CLAUDE.md` instructions.
2. Run `git status --short` and inspect recent commits; preserve any new user
   work in the song editor.
3. Start with custom layout preservation unless the user reprioritizes.
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
