// SPDX-License-Identifier: MIT

//! Standalone Bending Trainer: the Let's Bend-style harmonica bend diagram +
//! the metronome, with a directly pickable key and adjustable tempo — no
//! song. Its own [`AppState::BendingTrainer`](harmonicon_app::app::AppState), driving
//! the decoupled [`MetronomeTempo`] and its own copy of the gameplay clock
//! so the metronome ticks with nothing loaded. The harp is synthesised for
//! the chosen key (transposed Richter layout), rebuilding the diagram
//! whenever the key changes.
//!
//! A shared header (title top-left, Back button top-right — same shape as
//! every menu page, see `menu::scene::header_scene`/`spawn_back_button`)
//! sits above a two-column body, the same split `jam::session` uses: left
//! has everything but the harmonica itself, top-aligned and grouped into
//! five labelled sections — Setup (key, detect-algorithm), Practice Target
//! (readout/Listen/tuner, one card), Drill (toggle + its hover explanation),
//! Advanced (the collapsed precision drawer — see [`advanced`]) and Tempo
//! (metronome + BPM steppers) — instead of one flat stack, and each
//! group's own doc comment at its `setup` call site explains why (see
//! [`left_section`]); right is entirely the harmonica — the bend diagram
//! plus its technique hint.

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy::picking::events::{PointerClick, PointerOut, PointerOver};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use harmonicon_app::app::AppState;
use harmonicon_app::profile::{DrillRecord, PlayerProfile};
use harmonicon_audio::AudioSettings;
use harmonicon_audio::pitch_detect::{PITCH_RANGE_MARGIN_SEMITONES, PitchRange};
use harmonicon_core::harmonica::{Harmonica, HoleNotes, hole_notes, richter_harp};
use harmonicon_core::midi::{NOTE_NAMES, note_to_midi};
use harmonicon_core::wav::encode_wav;
use harmonicon_platform::localization::{Localization, LocalizationExt};
use harmonicon_platform::settings::BendingTrainerSettings;
use harmonicon_ui::dialogs::algo_picker::{algo_labels, attach_algo_tooltip, on_algo_selected};
use harmonicon_ui::dialogs::button;
use harmonicon_ui::dialogs::button::BaseButtonColor;
use harmonicon_ui::dialogs::combobox;
use harmonicon_ui::dialogs::combobox::ComboboxSelect;
use harmonicon_ui::dialogs::tooltip::Tooltip;

use std::collections::HashSet;

use super::harmonica_overlay::{
    CELL_DEFAULT, DiagramCellTarget, HarpOverlayCell, Row, spawn_harmonica_overlay_selectable,
};
use super::metronome_overlay::{MetronomeTempo, spawn_metronome};
use super::{ActivePitches, GameplayClock, GameplayRoot};
use harmonicon_ui::dialogs::page_chrome::{header_scene, spawn_back_button, title_column_scene};

const MIN_BPM: f32 = 40.0;
const MAX_BPM: f32 = 220.0;
const BPM_STEP: f32 = 5.0;

/// The key the trainer's diagram is currently built for.
#[derive(Resource)]
pub struct TrainerKey(pub String);

impl Default for TrainerKey {
    fn default() -> Self {
        Self("C".to_string())
    }
}

/// The 12 chromatic keys, as combobox option labels — `NOTE_NAMES` itself,
/// stringified.
fn key_labels() -> Vec<String> {
    NOTE_NAMES.iter().map(|s| s.to_string()).collect()
}

/// A combobox `on_select` that writes straight to [`TrainerKey`] — every
/// system that reacts to a key change (`rebuild_overlay`,
/// `update_pitch_range`) already keys off `TrainerKey::is_changed()`, so
/// picking a new key from the dropdown behaves exactly like the old
/// prev/next stepper did.
fn on_key_selected(ev: On<ComboboxSelect>, mut key: ResMut<TrainerKey>) {
    key.0 = ev.value.clone();
}

/// Wraps the harmonica diagram so it can be despawned + rebuilt on key change.
#[derive(Component)]
pub struct OverlayHost;

// ── Ear-training target ─────────────────────────────────────────────────────────

/// Which of the six technique rows in the diagram is the current target.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Technique {
    Blow,
    Draw,
    Bend1,
    Bend2,
    Bend3,
    Over,
}

/// Every technique row, in diagram order — used to enumerate drill targets.
const ALL_TECHNIQUES: [Technique; 6] = [
    Technique::Blow,
    Technique::Draw,
    Technique::Bend1,
    Technique::Bend2,
    Technique::Bend3,
    Technique::Over,
];

impl Technique {
    fn label_key(self, hole: u8) -> &'static str {
        match self {
            Technique::Blow => "bending-technique-blow",
            Technique::Draw => "bending-technique-draw",
            Technique::Bend1 => "bending-technique-bend-half",
            Technique::Bend2 => "bending-technique-bend-whole",
            Technique::Bend3 => "bending-technique-bend-three-half",
            Technique::Over if hole <= 6 => "bending-technique-overblow",
            Technique::Over => "bending-technique-overdraw",
        }
    }

    fn note(self, holes: &HoleNotes) -> Option<&str> {
        match self {
            Technique::Blow => holes.blow.as_deref(),
            Technique::Draw => holes.draw.as_deref(),
            Technique::Bend1 => holes.bends.first().map(String::as_str),
            Technique::Bend2 => holes.bends.get(1).map(String::as_str),
            Technique::Bend3 => holes.bends.get(2).map(String::as_str),
            Technique::Over => holes.over.as_deref(),
        }
    }

    /// Stable name used as the technique half of a `PlayerProfile::drills`
    /// key (`"{hole}:{technique}"`) — separate from [`label`](Self::label),
    /// which is player-facing display text free to change independently of
    /// what's already saved on disk.
    fn storage_key(self) -> &'static str {
        match self {
            Technique::Blow => "blow",
            Technique::Draw => "draw",
            Technique::Bend1 => "bend1",
            Technique::Bend2 => "bend2",
            Technique::Bend3 => "bend3",
            Technique::Over => "over",
        }
    }

    /// Inverse of [`storage_key`](Self::storage_key); `None` for anything
    /// else (e.g. a profile.json hand-edited or from a future version).
    fn from_storage_key(s: &str) -> Option<Self> {
        match s {
            "blow" => Some(Technique::Blow),
            "draw" => Some(Technique::Draw),
            "bend1" => Some(Technique::Bend1),
            "bend2" => Some(Technique::Bend2),
            "bend3" => Some(Technique::Bend3),
            "over" => Some(Technique::Over),
            _ => None,
        }
    }
}

/// The hole + technique the "Listen" button and the live tuner readout target.
/// Not every hole has every technique (e.g. hole 5 has no bend) — [`Technique::note`]
/// returns `None` for a hole/technique pair the harp can't produce.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub struct TrainerTarget {
    pub hole: u8,
    pub technique: Technique,
}

impl Default for TrainerTarget {
    fn default() -> Self {
        // Hole 2's half-step draw bend: the classic first bend most players learn.
        Self {
            hole: 2,
            technique: Technique::Bend1,
        }
    }
}

/// Maps a diagram [`Row`] to the [`Technique`] it represents — the diagram
/// distinguishes which *wing* a bend/over sits on (`BlowBend`/`DrawBend`,
/// `Overblow`/`Overdraw`, since that determines which reed it's read off),
/// but `Technique` resolves that from the hole number instead
/// (`Technique::note`), so several `Row`s collapse to one `Technique`.
/// `None` only for a `Row::*Bend` index outside 0..=2, which never actually
/// appears in [`super::harmonica_overlay`]'s `ROWS` table.
fn row_to_technique(row: Row) -> Option<Technique> {
    match row {
        Row::Blow => Some(Technique::Blow),
        Row::Draw => Some(Technique::Draw),
        Row::BlowBend(0) | Row::DrawBend(0) => Some(Technique::Bend1),
        Row::BlowBend(1) | Row::DrawBend(1) => Some(Technique::Bend2),
        Row::BlowBend(2) | Row::DrawBend(2) => Some(Technique::Bend3),
        Row::BlowBend(_) | Row::DrawBend(_) => None,
        Row::Overblow | Row::Overdraw => Some(Technique::Over),
    }
}

/// Sets the drill/ear-training target from a click on the harmonica diagram
/// — the picker itself, replacing the old hole/technique stepper buttons.
/// Shared across every selectable cell (see `spawn_harmonica_overlay_selectable`);
/// looks up which cell fired via `DiagramCellTarget` on the clicked entity
/// rather than a per-cell closure.
///
/// Under [`DrillScope::Custom`] the same click also toggles the cell's
/// membership in the drill's own pool, which is what makes that scope a
/// *set* the player builds rather than just whatever is selected right now.
/// The diagram is the only sensible place to pick cells, and it already
/// takes clicks — so a second interaction (a modifier chord, a separate
/// edit mode) would have cost discoverability for nothing. Removing a cell
/// leaves the target alone: taking something out of the pool is not a
/// request to go practice it.
// not-a-widget-button: harmonica-diagram cells are plain Nodes in a grid,
// not buttons — the keyboard path to a cell is the trainer's own key
// handling, not Tab focus.
fn on_diagram_cell_clicked(
    ev: On<PointerClick>,
    cells: Query<&DiagramCellTarget>,
    mut target: ResMut<TrainerTarget>,
    mut drill: ResMut<DrillState>,
) {
    let Ok(cell) = cells.get(ev.entity) else {
        return;
    };
    let Some(technique) = row_to_technique(cell.row) else {
        return;
    };
    if drill.scope == DrillScope::Custom && !drill.custom.insert((cell.hole, technique)) {
        drill.custom.remove(&(cell.hole, technique));
        return;
    }
    *target = TrainerTarget {
        hole: cell.hole,
        technique,
    };
}

/// Yellow-borders whichever diagram cell matches the current [`TrainerTarget`]
/// — the visible counterpart of [`on_diagram_cell_clicked`] — and, under
/// [`DrillScope::Custom`], dim-ambers every other cell in the custom pool so
/// the set the player is assembling is visible on the diagram itself rather
/// than only as a count in the scope readout.
///
/// Written every frame rather than gated on `target.is_changed()`: the
/// diagram itself is despawned and respawned on every key change
/// (`rebuild_overlay`), and a change-gated system would miss re-applying the
/// border to those fresh cells since the *target* didn't change, only the
/// diagram under it.
pub fn update_selected_cell_border(
    target: Res<TrainerTarget>,
    drill: Res<DrillState>,
    mut cells: Query<(&DiagramCellTarget, &mut BorderColor)>,
) {
    const SELECTED: Color = Color::srgb(0.95, 0.85, 0.20);
    const IN_CUSTOM_SCOPE: Color = Color::srgb(0.55, 0.45, 0.16);
    let show_custom = drill.scope == DrillScope::Custom;
    for (cell, mut border) in &mut cells {
        let technique = row_to_technique(cell.row);
        let color = if cell.hole == target.hole && technique == Some(target.technique) {
            SELECTED
        } else if show_custom && technique.is_some_and(|t| drill.custom.contains(&(cell.hole, t))) {
            IN_CUSTOM_SCOPE
        } else {
            Color::NONE
        };
        let wanted = BorderColor::all(color);
        if *border != wanted {
            *border = wanted;
        }
    }
}

/// The "Target: Hole N Draw" readout.
#[derive(Component)]
pub struct TargetLabel;

/// The "how to physically play this" hint box, kept in step with the target.
#[derive(Component)]
pub struct HintLabel;

/// The live cents-off tuner readout.
#[derive(Component)]
pub struct TunerReadout;

#[derive(Resource, Default)]
pub struct NaturalCheck {
    requested: bool,
    hold_secs: f32,
    confirmed: bool,
    /// Per-frame deviation from the table, in cents, of the samples accepted
    /// so far — averaged into the reed's observed centre when the check
    /// confirms (`feedback::observed_center_cents`).
    samples: Vec<f32>,
}

#[derive(Component)]
pub struct NaturalCheckLabel;

/// The drill's "on/off" readout, plus a running streak/weak-spot summary.
#[derive(Component)]
pub struct DrillLabel;

/// The Drill toggle button itself — tagged so [`update_drill_button_visual`]
/// can highlight it while the drill is running, since the adjacent
/// [`DrillLabel`] text alone is easy to miss (a toggle should look pressed,
/// not just say so nearby).
#[derive(Component)]
pub struct DrillToggleButton;

/// The Drill button's hover explanation; empty while not hovering it.
#[derive(Component)]
pub struct DrillExplanation;

/// Practical "how do I actually play this" text for a technique on a given
/// hole. Bends and overs go a different physical direction depending on
/// which side of the harp the hole is on, so both are needed to be accurate:
/// holes 1\u{2013}6 bend (and overblow) by drawing, holes 7\u{2013}10 by blowing.
fn technique_hint_key(technique: Technique, hole: u8) -> &'static str {
    match technique {
        Technique::Blow => "bending-technique-hint-blow",
        Technique::Draw => "bending-technique-hint-draw",
        Technique::Bend1 if hole <= 6 => "bending-technique-hint-draw-bend-half",
        Technique::Bend2 if hole <= 6 => "bending-technique-hint-draw-bend-whole",
        Technique::Bend3 if hole <= 6 => "bending-technique-hint-draw-bend-three-half",
        Technique::Bend1 => "bending-technique-hint-blow-bend-half",
        Technique::Bend2 => "bending-technique-hint-blow-bend-whole",
        Technique::Bend3 => "bending-technique-hint-blow-bend-three-half",
        Technique::Over => match hole {
            1 | 4 | 5 | 6 => "bending-technique-hint-overblow",
            7..=10 => "bending-technique-hint-overdraw",
            _ => "bending-technique-hint-over-unsupported",
        },
    }
}

fn technique_hint(loc: &Localization, technique: Technique, hole: u8) -> String {
    String::from(loc.msg_args(
        technique_hint_key(technique, hole),
        &[("hole", hole.to_string())],
    ))
}

/// The pitch detector's search range for `key`'s transposed Richter harp,
/// widened by a semitone margin — the trainer's own key-derived range, kept
/// separate from a loaded chart's (see `setup_scoring_config` in `mod.rs`).
fn pitch_range_for_key(key: &str) -> PitchRange {
    richter_harp(key)
        .frequency_range()
        .map(|(lo, hi)| PitchRange::from_freqs([lo, hi], PITCH_RANGE_MARGIN_SEMITONES))
        .unwrap_or_default()
}

/// A small muted "eyebrow" label marking the start of a left-panel control
/// group (Setup / Practice Target / Drill / Tempo) — purely a visual
/// grouping cue, not an interactive widget, so a first glance at the panel
/// shows four short groups instead of one flat stack of six unrelated rows.
fn left_section(parent: &mut ChildSpawnerCommands, text: &str) {
    parent.spawn((
        Text::new(text.to_string()),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgb(0.45, 0.45, 0.55)),
    ));
}

// ── Lifecycle ─────────────────────────────────────────────────────────────────

pub fn setup(
    mut commands: Commands,
    mut clock: ResMut<GameplayClock>,
    mut tempo: ResMut<MetronomeTempo>,
    key: Res<TrainerKey>,
    target: Res<TrainerTarget>,
    audio: Res<AudioSettings>,
    mut pitch_range: ResMut<PitchRange>,
    mut drill: ResMut<DrillState>,
    profile: Res<PlayerProfile>,
    settings: Res<BendingTrainerSettings>,
    loc: Res<Localization>,
) {
    clock.set_free(0.0);
    *pitch_range = pitch_range_for_key(&key.0);
    tempo.meter = harmonicon_ui::music_score::MusicScoreMeter::default();
    // Keep whatever BPM was last set; default to a comfortable practice tempo.
    if tempo.bpm < MIN_BPM || tempo.bpm > MAX_BPM {
        tempo.bpm = 90.0;
    }
    // Restore drill hit-rates from the last session — see `save_drill_progress`,
    // which persists them on the way out.
    drill.stats = stats_from_profile(&profile.drills);

    let root_id = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.08)),
            GameplayRoot,
        ))
        .id();

    // Header: title top-left, Back button top-right — the same shared
    // header shape every menu page uses (`menu::scene::header_scene`/
    // `title_column_scene`/`spawn_back_button`), composed directly rather
    // than through `spawn_menu_root` since this screen isn't a menu page
    // and doesn't want its background image/scroll-area/`MenuRoot` cleanup
    // tag. Not separately tagged `GameplayRoot` — despawning `root_id` on
    // exit (`cleanup_gameplay`) already recurses into every child.
    let title_column = commands
        .spawn_scene(title_column_scene(String::from(loc.msg("bending-trainer"))))
        .id();
    let header = commands.spawn_scene(header_scene()).id();
    commands.entity(header).add_child(title_column);
    commands.entity(root_id).add_child(header);
    spawn_back_button(
        &mut commands,
        header,
        &loc.msg("back"),
        |_: On<Activate>,
         mut next_state: ResMut<NextState<AppState>>,
         mut ret_play: ResMut<harmonicon_app::app::ReturnToPlay>| {
            ret_play.0 = true;
            next_state.set(AppState::Menu);
        },
    );

    // Body: the two-column layout below the header, filling the rest of
    // the screen's height.
    let body = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            ..default()
        })
        .id();
    commands.entity(root_id).add_child(body);
    // Captured so the Detect-algorithm combobox below can pass it as the
    // *backdrop*'s parent — `combobox::spawn_combobox` requires a
    // full-screen-sized backdrop parent for its click-catching backdrop to
    // size correctly (see its module doc comment), so a click anywhere on
    // the right column (not just the left) still dismisses an open dropdown.
    // The combobox's visible trigger, meanwhile, is parented to the left
    // column itself so it sits in that column's normal vertical flow.
    commands.entity(body).with_children(|root| {
        // ── Left half: everything but the harmonica itself, grouped into
        // four labelled sections (Setup / Practice Target / Drill / Tempo)
        // instead of one flat stack — see this module's own doc comment.
        // Top-aligned (not centered): a centered column recenters its
        // *whole* stack every time any one row's content changes height
        // (the drill explanation appearing/disappearing, a longer/shorter
        // tuner reading), which visibly shifts every other control:
        // top-aligned means a height change only ever pushes content below
        // it, never the rows above.
        let mut left_ec = root.spawn(Node {
            width: Val::Percent(50.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::FlexStart,
            row_gap: Val::Px(22.0),
            padding: UiRect::all(Val::Px(16.0)),
            ..default()
        });
        let left_id = left_ec.id();
        left_ec.with_children(|left| {
            // ── Setup: key + detect algorithm ───────────────────────────────
            left_section(left, &loc.msg("bending-section-setup"));
            combobox::spawn_combobox(
                left.commands_mut(),
                left_id,
                root_id,
                &loc.msg("bending-key-label"),
                &key_labels(),
                &key.0,
                on_key_selected,
            );
            let algo_combo = combobox::spawn_combobox(
                left.commands_mut(),
                left_id,
                root_id,
                &loc.msg("bending-detect-label"),
                &algo_labels(&loc),
                audio.pitch_algorithm.label(),
                on_algo_selected,
            );
            attach_algo_tooltip(left.commands_mut(), algo_combo, audio.pitch_algorithm);

            // ── Practice Target: readout, Listen, and the live tuner,
            // grouped into one card (same background the technique-hint
            // card in the right column uses) so the three read as a single
            // "what am I practicing right now" unit instead of floating as
            // bare, visually disconnected rows ──────────────────────────────
            left_section(left, &loc.msg("bending-section-target"));
            left.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Px(320.0),
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.10, 0.10, 0.14, 0.85)),
            ))
            .with_children(|card| {
                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                        Text::new(target_label_text(&loc, target.hole, target.technique)),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.80, 0.90, 0.95)),
                        TargetLabel,
                    ));
                    row.spawn_empty().apply_scene(button::small(
                        &loc.msg("bending-listen-natural-button"),
                        |_: On<Activate>,
                         key: Res<TrainerKey>,
                         target: Res<TrainerTarget>,
                         mut sources: ResMut<Assets<AudioSource>>,
                         mut commands: Commands| {
                            let harp = richter_harp(&key.0);
                            let Some(note) = natural_note_for_target(&harp, *target) else {
                                return;
                            };
                            play_reference_note(&note, &mut sources, &mut commands);
                        },
                    ));
                    row.spawn_empty().apply_scene(button::small(
                        &loc.msg("bending-listen-target-button"),
                        |_: On<Activate>,
                         key: Res<TrainerKey>,
                         target: Res<TrainerTarget>,
                         mut sources: ResMut<Assets<AudioSource>>,
                         mut commands: Commands| {
                            let harp = richter_harp(&key.0);
                            let Some(note) = target_note(&harp, *target) else {
                                return;
                            };
                            play_reference_note(&note, &mut sources, &mut commands);
                        },
                    ));
                });
                card.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.55, 0.85, 0.60)),
                    TunerReadout,
                ));
                spawn_bend_rail(card, &loc);
                card.spawn_empty().apply_scene(button::small(
                    &loc.msg("bending-check-natural-button"),
                    |_: On<Activate>, mut check: ResMut<NaturalCheck>| {
                        *check = NaturalCheck {
                            requested: true,
                            ..default()
                        };
                    },
                ));
                card.spawn((
                    Text::new(String::from(
                        loc.msg_args(
                            "bending-check-natural-idle",
                            &[(
                                "note",
                                natural_note_for_target(&richter_harp(&key.0), *target)
                                    .unwrap_or_else(|| "?".to_string()),
                            )],
                        ),
                    )),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.60, 0.60, 0.70)),
                    NaturalCheckLabel,
                ));
            });

            // ── Drill: adaptive ear-training loop. The toggle button is
            // tagged `DrillToggleButton` so `update_drill_button_visual`
            // can highlight it while running — the on/off state used to be
            // carried by the small `DrillLabel` text alone, easy to miss at
            // a glance. Its hover explanation now sits directly below the
            // button itself, not across in the right column (where hovering
            // a left-column control used to change text nowhere near the
            // pointer) ───────────────────────────────────────────────────────
            left_section(left, &loc.msg("bending-section-drill"));
            left.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn_empty().apply_scene(button::small(
                    &loc.msg("bending-scope-button"),
                    cycle_drill_scope,
                ));
                row.spawn((
                    Text::new(scope_status(&loc, DrillScope::default(), 0)),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.70, 0.70, 0.80)),
                    DrillScopeLabel,
                ));
            });
            left.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn_empty().apply_scene(button::small(
                    &loc.msg("bending-shape-button"),
                    cycle_practice_shape,
                ));
                row.spawn((
                    Text::new(String::from(loc.msg("bending-shape-free"))),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.70, 0.70, 0.80)),
                    PracticeShapeLabel,
                ));
            });
            left.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn_empty()
                    .apply_scene(button::small(
                        &loc.msg("bending-drill-button"),
                        |_: On<Activate>,
                         key: Res<TrainerKey>,
                         mut target: ResMut<TrainerTarget>,
                         mut drill: ResMut<DrillState>| {
                            drill.enabled = !drill.enabled;
                            drill.hold_secs = 0.0;
                            drill.elapsed_secs = 0.0;
                            drill.attempted = false;
                            if drill.enabled {
                                let harp = richter_harp(&key.0);
                                if let Some(next) = pick_next_target(
                                    &harp,
                                    &drill.stats,
                                    Some(*target),
                                    drill.scope,
                                    &drill.custom,
                                    *target,
                                ) {
                                    *target = next;
                                }
                            }
                        },
                    ))
                    .insert(DrillToggleButton)
                    .observe(show_drill_explanation)
                    .observe(hide_drill_explanation);
                row.spawn((
                    Text::new(String::from(loc.msg("bending-drill-off"))),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.70, 0.70, 0.80)),
                    DrillLabel,
                ));
            });
            left.spawn((
                Node {
                    width: Val::Px(320.0),
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.60, 0.60, 0.70)),
                DrillExplanation,
            ));

            // ── Advanced: the precision controls and the measurement
            // view, collapsed by default (see `advanced`'s module doc) ──────
            spawn_advanced_drawer(left, &loc, &settings);

            // ── Tempo control: −  ♩ = NN (in the metronome)  + ──────────────
            left_section(left, &loc.msg("bending-section-tempo"));
            left.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn_empty()
                    .apply_scene(button::small(
                        "\u{2212}",
                        |_: On<Activate>, mut tempo: ResMut<MetronomeTempo>| {
                            tempo.bpm = (tempo.bpm - BPM_STEP).max(MIN_BPM);
                        },
                    ))
                    .insert(Tooltip(String::from(loc.msg("bending-tempo-decrease"))));
                row.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|metro| {
                    spawn_metronome(metro, &loc, tempo.beats_per_bar(), tempo.bpm);
                });
                row.spawn_empty()
                    .apply_scene(button::small(
                        "+",
                        |_: On<Activate>, mut tempo: ResMut<MetronomeTempo>| {
                            tempo.bpm = (tempo.bpm + BPM_STEP).min(MAX_BPM);
                        },
                    ))
                    .insert(Tooltip(String::from(loc.msg("bending-tempo-increase"))));
            });

            left.spawn((
                Text::new(String::from(loc.msg("bending-hint"))),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.55, 0.65)),
            ));
        });

        // ── Right half: the harmonica — bend diagram + its explanatory
        // text, the same grouping `jam::session::setup` uses for its own
        // harmonica column.
        root.spawn(Node {
            width: Val::Percent(50.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(10.0),
            padding: UiRect::all(Val::Px(16.0)),
            ..default()
        })
        .with_children(|right| {
            // The bend diagram (rebuilt on key change).
            right
                .spawn((Node::default(), OverlayHost))
                .with_children(|host| {
                    spawn_harmonica_overlay_selectable(
                        host,
                        &richter_harp(&key.0),
                        on_diagram_cell_clicked,
                        &loc,
                    );
                });

            right
                .spawn((
                    Node {
                        width: Val::Px(280.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.10, 0.10, 0.14, 0.85)),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new(technique_hint(&loc, target.technique, target.hole)),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.75, 0.75, 0.85)),
                        HintLabel,
                    ));
                });
        });
    });
}

/// Advance the trainer's own clock (no song to drive it).
pub fn tick_clock(mut clock: ResMut<GameplayClock>, time: Res<Time>) {
    clock.advance(time.delta_secs_f64(), None);
}

/// Rebuild the bend diagram when the key changes.
pub fn rebuild_overlay(
    key: Res<TrainerKey>,
    hosts: Query<(Entity, Option<&Children>), With<OverlayHost>>,
    mut commands: Commands,
    loc: Res<Localization>,
) {
    if !key.is_changed() {
        return;
    }
    let harp = richter_harp(&key.0);
    for (host, children) in &hosts {
        if let Some(children) = children {
            for &c in children {
                commands.entity(c).despawn();
            }
        }
        commands.entity(host).with_children(|h| {
            spawn_harmonica_overlay_selectable(h, &harp, on_diagram_cell_clicked, &loc);
        });
    }
}

/// Re-derive the pitch detector's range when the key changes.
pub fn update_pitch_range(key: Res<TrainerKey>, mut pitch_range: ResMut<PitchRange>) {
    if !key.is_changed() {
        return;
    }
    *pitch_range = pitch_range_for_key(&key.0);
}

/// Esc returns to the menu — specifically the Play page, where "Bending
/// Trainer" lives, rather than `MenuPage`'s own default of Main.
pub fn handle_escape(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut ret_play: ResMut<harmonicon_app::app::ReturnToPlay>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        ret_play.0 = true;
        next_state.set(AppState::Menu);
    }
}

/// "Target: Hole 2 · ½-step bend" — or a note that the current harp can't
/// actually produce there, so the reader knows why Listen did nothing.
fn target_label_text(loc: &Localization, hole: u8, technique: Technique) -> String {
    let technique = loc.msg(technique.label_key(hole));
    String::from(loc.msg_args(
        "bending-target-label",
        &[
            ("hole", hole.to_string()),
            ("technique", technique.to_string()),
        ],
    ))
}

/// Keep the "Target: ..." readout in step with the chosen hole/technique.
pub fn update_target_label(
    target: Res<TrainerTarget>,
    loc: Res<Localization>,
    mut labels: Query<&mut Text, With<TargetLabel>>,
) {
    if !target.is_changed() {
        return;
    }
    for mut text in &mut labels {
        *text = Text::new(target_label_text(&loc, target.hole, target.technique));
    }
}

/// Keep the how-to-play hint in step with the chosen hole/technique.
pub fn update_hint_label(
    target: Res<TrainerTarget>,
    loc: Res<Localization>,
    mut labels: Query<&mut Text, With<HintLabel>>,
) {
    if !target.is_changed() {
        return;
    }
    for mut text in &mut labels {
        *text = Text::new(technique_hint(&loc, target.technique, target.hole));
    }
}

/// Show what Drill mode does while the button is hovered.
/// What Drill mode actually does, shown only while hovering the button —
/// it's not obvious from the label alone that it's adaptive/weighted.
fn show_drill_explanation(
    _: On<PointerOver>,
    loc: Res<Localization>,
    mut labels: Query<&mut Text, With<DrillExplanation>>,
) {
    for mut text in &mut labels {
        *text = Text::new(String::from(loc.msg("bending-drill-explanation")));
    }
}

/// Hide the Drill explanation once the pointer leaves the button.
fn hide_drill_explanation(_: On<PointerOut>, mut labels: Query<&mut Text, With<DrillExplanation>>) {
    for mut text in &mut labels {
        *text = Text::new("");
    }
}

mod advanced;
mod drill;
mod feedback;
mod gesture;
#[cfg(test)]
mod tests;
mod trace;

pub use advanced::*;
pub use drill::*;
pub use feedback::*;
pub use gesture::*;
pub use trace::*;
