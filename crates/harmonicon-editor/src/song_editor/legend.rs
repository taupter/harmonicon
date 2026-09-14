// SPDX-License-Identifier: MIT

//! The meta form's third column: a colour legend naming what every colour
//! the grid, mod panel and scrollbar uses actually means, plus the system
//! that shows and hides it.
//!
//! Split out of `meta_form` for that file's line budget
//! (`docs/physical_design_plan.md`) — this is read-only explanatory UI with
//! no field editing in it, which made it the cleanest seam.

use bevy::prelude::*;

use super::grid::{OUT_OF_SCALE_MIX, OUT_OF_SCALE_TINT, TEMPO_MARKER_COLOR, mix_srgba};
use super::state::{Dir, EditorState, Pitch, pitch_color};
use super::timeline_overlay::{RANGE_HIGHLIGHT_COLOR, SPLIT_LINE_COLOR};
use super::ui::LegendColumn;
use super::view_scroll::{SCROLLBAR_BLOW_COLOR, SCROLLBAR_DRAW_COLOR};
use bevy_fluent::prelude::Localization;
use harmonicon_platform::localization::LocalizationExt;
use harmonicon_platform::theme::SongEditorColors;

/// Shows/hides the meta form's third (legend) column to match
/// `EditorState::legend_visible` — toggled by the mod panel's "ℹ Legend"
/// button (`mod_panel.rs`).
pub(super) fn update_legend_visibility(
    state: Res<EditorState>,
    mut columns: Query<&mut Node, With<LegendColumn>>,
) {
    if !state.is_changed() {
        return;
    }
    for mut node in &mut columns {
        node.display = if state.legend_visible {
            Display::Flex
        } else {
            Display::None
        };
    }
}

// ── Color legend ──────────────────────────────────────────────────────────────

/// A legend swatch's fixed size — small enough to sit beside its label like
/// a bullet, big enough that the color itself (not just its position) reads
/// clearly.
const SWATCH_SIZE: f32 = 16.0;

/// One legend entry: a color swatch (a plain filled box, or — via
/// `border_only` — an unfilled box with just a colored border, for the
/// entries that are actually borders in the real UI, not fills) plus its
/// explanation.
pub(super) fn spawn_legend_row(
    col: &mut ChildSpawnerCommands,
    colors: SongEditorColors,
    swatch: Color,
    border_only: bool,
    text: String,
) {
    col.spawn(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|line| {
        line.spawn((
            Node {
                width: Val::Px(SWATCH_SIZE),
                height: Val::Px(SWATCH_SIZE),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(if border_only { 2.0 } else { 1.0 })),
                ..default()
            },
            BackgroundColor(if border_only { Color::NONE } else { swatch }),
            BorderColor::all(if border_only {
                swatch
            } else {
                Color::srgb(0.30, 0.30, 0.40)
            }),
        ));
        line.spawn((
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(12.5),
                ..default()
            },
            TextColor(colors.label),
        ));
    });
}

/// A small section heading within the legend column — same accent color
/// the rest of the editor uses for anything meant to draw the eye.
pub(super) fn spawn_legend_heading(
    col: &mut ChildSpawnerCommands,
    colors: SongEditorColors,
    text: String,
) {
    col.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(colors.accent),
        Node {
            margin: UiRect::top(Val::Px(4.0)),
            ..default()
        },
    ));
}

/// Explains every color the song editor uses, grouped by where it shows up.
/// A grid note's *fill* color is its playing technique (blow vs. draw is
/// the small ↑/↓ arrow, not a color), while the scrollbar minimap's
/// blue/orange markers mean blow/draw specifically — the same blue means
/// two different things in two different places, worth spelling out rather
/// than making the player reverse-engineer `theme.json`.
pub(super) fn spawn_color_legend(
    col: &mut ChildSpawnerCommands,
    loc: &Localization,
    colors: SongEditorColors,
) {
    spawn_legend_heading(col, colors, loc.msg("editor-legend-notes").to_string());
    spawn_legend_row(
        col,
        colors,
        pitch_color(Pitch::Normal),
        false,
        loc.msg("editor-legend-normal").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        pitch_color(Pitch::Bend(1.0)),
        false,
        loc.msg("editor-legend-bend").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        pitch_color(Pitch::Overblow),
        false,
        loc.msg("editor-legend-overblow").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        pitch_color(Pitch::Overdraw),
        false,
        loc.msg("editor-legend-overdraw").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        pitch_color(Pitch::Slide),
        false,
        loc.msg("editor-legend-slide").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        mix_srgba(
            pitch_color(Pitch::Normal),
            OUT_OF_SCALE_TINT,
            OUT_OF_SCALE_MIX,
        ),
        false,
        loc.msg("editor-legend-out-of-scale").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        colors.accent,
        true,
        loc.msg("editor-legend-selected").to_string(),
    );
    col.spawn((
        Text::new(format!(
            "{}  {} / {}  {}",
            Dir::Blow.arrow(),
            loc.msg("editor-legend-blow"),
            loc.msg("editor-legend-draw"),
            Dir::Draw.arrow(),
        )),
        TextFont {
            font_size: FontSize::Px(12.5),
            ..default()
        },
        TextColor(colors.label),
    ));

    spawn_legend_heading(col, colors, loc.msg("editor-legend-dragging").to_string());
    spawn_legend_row(
        col,
        colors,
        colors.ghost_ok.with_alpha(0.30),
        false,
        loc.msg("editor-legend-drag-ok").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        colors.ghost_bad.with_alpha(0.30),
        false,
        loc.msg("editor-legend-drag-bad").to_string(),
    );

    spawn_legend_heading(col, colors, loc.msg("editor-legend-elsewhere").to_string());
    spawn_legend_row(
        col,
        colors,
        TEMPO_MARKER_COLOR,
        false,
        loc.msg("editor-legend-tempo-marker").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        colors.triplet_line,
        false,
        loc.msg("editor-legend-triplet-line").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        SPLIT_LINE_COLOR,
        false,
        loc.msg("editor-legend-split-point").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        RANGE_HIGHLIGHT_COLOR,
        false,
        loc.msg("editor-legend-range-preview").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        colors.btn_active,
        false,
        loc.msg("editor-legend-active-button").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        SCROLLBAR_BLOW_COLOR,
        false,
        loc.msg("editor-legend-scrollbar-blow").to_string(),
    );
    spawn_legend_row(
        col,
        colors,
        SCROLLBAR_DRAW_COLOR,
        false,
        loc.msg("editor-legend-scrollbar-draw").to_string(),
    );
    col.spawn((
        Text::new(loc.msg("editor-legend-scrollbar-note").to_string()),
        TextFont {
            font_size: FontSize::Px(10.0),
            ..default()
        },
        TextColor(colors.label.with_alpha(0.75)),
        Node {
            max_width: Val::Px(220.0),
            ..default()
        },
    ));
}
