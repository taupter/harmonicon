// SPDX-License-Identifier: MIT

//! Persistent visibility state and compact readouts for Jam Session's
//! listening-first stage.

use bevy::prelude::*;

use harmonicon_gameplay::gameplay::CurrentBar;
use harmonicon_platform::localization::{Localization, LocalizationExt};

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

pub fn update_jam_guides(
    guides: Res<JamGuidesVisible>,
    loc: Res<Localization>,
    mut panels: Query<&mut Visibility, With<JamGuidePanel>>,
    mut primary: Query<&mut Node, With<JamPrimaryPanel>>,
    mut labels: Query<&mut Text, With<JamGuidesLabel>>,
) {
    if !guides.is_changed() {
        return;
    }
    for mut visibility in &mut panels {
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
