// SPDX-License-Identifier: MIT

//! The static furniture of the 2D play area: the highway's lane stripes and
//! hit zone, the harmonica hole strip beneath it, and the blow/draw key.
//!
//! Split out of `gameplay_2d` because it is the half that runs *once*, at
//! song setup, and never again — the rest of that module spawns, moves and
//! retires note visuals every frame. The two share only the lane geometry,
//! and share it by construction: both derive lane width from
//! `100.0 / hole_count`, which is what keeps a hole number under its own
//! lane on any harp, 10-hole diatonic or 12-hole chromatic.

use bevy::prelude::*;

use harmonicon_core::chart::Action;
use harmonicon_platform::localization::{Localization, LocalizationExt};

use super::HoleCell;
use super::gameplay_2d::HIT_H_PCT;

/// Spawns the static highway furniture (lane stripes, dividers, hit zone) —
/// no notes. Notes are spawned later, lazily, by `spawn_visible_notes`.
pub(super) fn spawn_highway(
    hw: &mut ChildSpawnerCommands,
    chart: &harmonicon_core::chart::HarpChart,
) {
    // Lane count/width come from the loaded harmonica, not a fixed 10 —
    // a chromatic chart's 12+ holes need proportionally narrower lanes.
    let hole_count = chart.harmonica.hole_count() as usize;
    let lane_pct = 100.0 / hole_count as f32;

    for h in 0..hole_count {
        let left_pct = h as f32 * lane_pct;
        let alpha = if h % 2 == 0 { 0.04f32 } else { 0.0f32 };
        hw.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(left_pct),
                top: Val::Percent(0.0),
                width: Val::Percent(lane_pct),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, alpha)),
        ));
        if h > 0 {
            hw.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(left_pct),
                    top: Val::Percent(0.0),
                    width: Val::Px(1.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
            ));
        }
    }

    // Hit zone: a band a note's head must reach, and the line at its top
    // edge that is the actual judgment instant.
    hw.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(0.0),
            bottom: Val::Percent(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(HIT_H_PCT),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 0.55, 0.14)),
    ));
    hw.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(0.0),
            bottom: Val::Percent(HIT_H_PCT),
            // Full width, and warmer and heavier than a beat guide. This is
            // the one line on the highway that means "now"; the guides
            // crossing it are white, thinner and far dimmer, so the two
            // can't be mistaken for each other as they scroll past.
            width: Val::Percent(100.0),
            height: Val::Px(3.0),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 0.98, 0.62, 0.85)),
    ));
}

pub(super) fn spawn_harmonica_strip(
    col: &mut ChildSpawnerCommands,
    chart: &harmonicon_core::chart::HarpChart,
    loc: &Localization,
) {
    let hole_count = chart.harmonica.hole_count();
    let lane_pct = 100.0 / hole_count as f32;
    col.spawn(Node {
        flex_direction: FlexDirection::Row,
        width: Val::Percent(100.0),
        ..default()
    })
    .with_children(|row| {
        for hole in 1u8..=hole_count {
            let b = chart.harmonica.wind_direction_label(hole, &Action::Blow);
            let d = chart.harmonica.wind_direction_label(hole, &Action::Draw);
            row.spawn((
                Node {
                    width: Val::Percent(lane_pct),
                    // Fixed px, not Vh — Vh resolves from the physical
                    // viewport and doesn't respond to `UiScale`, unlike this
                    // cell's own text, so the cell would stay a fixed size on
                    // screen while its labels scaled independently.
                    height: Val::Px(96.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceAround,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.10, 0.12, 0.16)),
                BorderColor::all(Color::srgb(0.28, 0.30, 0.40)),
                HoleCell(hole),
            ))
            .with_children(|cell| {
                cell.spawn((
                    Text::new(b),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.50, 0.75, 1.00)),
                ));
                cell.spawn((
                    Text::new(format!("{hole}")),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                cell.spawn((
                    Text::new(d),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.00, 0.62, 0.35)),
                ));
            });
        }
    });

    spawn_blow_draw_legend(col, loc, 20.0, 0.0);
}

/// The "Blow / Draw" colour-key legend — identical between the 2D harmonica
/// strip and the 3D HUD overlay, differing only in the row's own spacing
/// (`column_gap`/top `margin`, which each caller picks to match its
/// surrounding layout).
pub(super) fn spawn_blow_draw_legend(
    parent: &mut ChildSpawnerCommands,
    loc: &Localization,
    column_gap: f32,
    margin_top: f32,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(column_gap),
            margin: UiRect::top(Val::Px(margin_top)),
            ..default()
        })
        .with_children(|leg| {
            leg.spawn((
                Text::new(String::from(loc.msg("gameplay-legend-blow"))),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.50, 0.75, 1.00)),
            ));
            leg.spawn((
                Text::new(String::from(loc.msg("gameplay-legend-draw"))),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(1.00, 0.62, 0.35)),
            ));
        });
}
