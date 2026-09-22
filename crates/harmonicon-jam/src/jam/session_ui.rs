// SPDX-License-Identifier: MIT

//! Persistent visibility state and compact readouts for Jam Session's
//! listening-first stage.

use bevy::prelude::*;

use harmonicon_core::harmonica::Progression;
use harmonicon_gameplay::gameplay::CurrentBar;
use harmonicon_platform::localization::{Localization, LocalizationExt};
use harmonicon_platform::theme::TwelveBarColors;
use harmonicon_ui::dialogs::twelve_bar_grid::{BarCell, bar_bg};

#[derive(Resource, Default)]
pub struct JamGuidesVisible(pub bool);

#[derive(Resource, Default)]
pub struct JamChordSequence(pub Vec<String>);

#[derive(Component)]
pub struct JamGuidePanel;

#[derive(Component)]
pub struct JamPrimaryPanel;

#[derive(Component)]
pub struct JamGuidesLabel;

#[derive(Component)]
pub struct JamChordPosition;

/// Shows or collapses every guide panel and hands the primary stage the
/// width the guides give back. Collapsing is `Display::None`, not just
/// `Visibility::Hidden`: a hidden node still takes its space, which left
/// the stage centred in the left two thirds of the window and its stack
/// pinned to the top. Runs when the toggle changes and on the frame the
/// panels are spawned, so a jam that opens with guides off lays out
/// correctly from the start.
pub fn update_jam_guides(
    guides: Res<JamGuidesVisible>,
    loc: Res<Localization>,
    added: Query<(), Added<JamGuidePanel>>,
    mut panels: Query<(&mut Node, &mut Visibility), With<JamGuidePanel>>,
    mut primary: Query<&mut Node, (With<JamPrimaryPanel>, Without<JamGuidePanel>)>,
    mut labels: Query<&mut Text, With<JamGuidesLabel>>,
) {
    if !guides.is_changed() && added.is_empty() {
        return;
    }
    for (mut node, mut visibility) in &mut panels {
        node.display = if guides.0 {
            Display::Flex
        } else {
            Display::None
        };
        *visibility = if guides.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut node in &mut primary {
        node.width = Val::Percent(if guides.0 { 50.0 } else { 100.0 });
    }
    for mut text in &mut labels {
        *text = Text::new(String::from(if guides.0 {
            loc.msg("jam-guides-on")
        } else {
            loc.msg("jam-guides-off")
        }));
    }
}

/// The compact form strip on the default stage: twelve small cells in three
/// groups of four, coloured by chord function like the full grid and
/// tagged `BarCell` so `gameplay::twelve_bar_blues_overlay::update_bar`
/// lights the current one exactly as it lights the grid's — one bar clock,
/// two views of it.
pub fn spawn_form_strip(
    parent: &mut ChildSpawnerCommands,
    key: &str,
    progression: Progression,
    colors: TwelveBarColors,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|strip| {
            for line in 0..3usize {
                strip
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(3.0),
                        ..default()
                    })
                    .with_children(|group| {
                        for col in 0..4usize {
                            let bar = line * 4 + col;
                            group.spawn((
                                Node {
                                    width: Val::Px(26.0),
                                    height: Val::Px(12.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(bar_bg(bar, key, progression, colors)),
                                BorderColor::all(Color::srgb(0.25, 0.25, 0.38)),
                                BarCell(bar),
                            ));
                        }
                    });
            }
        });
}

pub fn update_jam_chord_position(
    current: Res<CurrentBar>,
    chords: Res<JamChordSequence>,
    mut labels: Query<&mut Text, With<JamChordPosition>>,
) {
    if !current.is_changed() && !chords.is_changed() {
        return;
    }
    let Some(now) = chords.0.get(current.0 % chords.0.len().max(1)) else {
        return;
    };
    let next = &chords.0[(current.0 + 1) % chords.0.len()];
    for mut text in &mut labels {
        *text = Text::new(format!("{now}  →  {next}"));
    }
}
