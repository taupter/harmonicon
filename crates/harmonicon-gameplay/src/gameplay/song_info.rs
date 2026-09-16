// SPDX-License-Identifier: MIT

//! What the HUD says about the *song* rather than about the performance:
//! title, key/tempo/meter, which harp to hold, the chart's description and
//! author.
//!
//! Resolved once at song setup into [`SongInfo`] and read from there by every
//! screen that shows it, rather than each one re-deriving it from the chart.
//! Both gameplay modes built this block independently and identically before,
//! down to the same em dash and the same `metadata.as_ref().and_then(...)`.
//!
//! **It is read material, not a readout.** None of it changes during a
//! performance, so it belongs where the player has time to read it — the
//! countdown and the pause menu — with only the title kept on screen while
//! notes are falling. See [`spawn_song_details`] and [`spawn_song_header`].

use bevy::prelude::*;
use bevy_fluent::Localization;

use harmonicon_core::chart::HarpChart;
use harmonicon_platform::localization::LocalizationExt;

/// Every song-describing string the HUD needs, resolved and localized once.
#[derive(Resource, Default, Clone)]
pub struct SongInfo {
    /// `"Artist — Title"`.
    pub title: String,
    /// `"Key: C ♩ = 80 3/4"`.
    pub meter: String,
    /// `"Diatonic · 10 holes · 1st position · Richter"`.
    pub harp: String,
    pub description: Option<String>,
    /// Already localized as `"Chart: {author}"`, not the bare name.
    pub chart_author: Option<String>,
}

impl SongInfo {
    pub fn from_chart(chart: &HarpChart, loc: &Localization) -> Self {
        Self {
            title: format!("{} \u{2014} {}", chart.song.artist, chart.song.title),
            meter: String::from(
                loc.msg_args(
                    "gameplay-chart-info",
                    &[
                        ("key", chart.song.key.clone()),
                        ("bpm", (chart.song.tempo_bpm as u32).to_string()),
                        (
                            "time_sig",
                            chart
                                .song
                                .time_signature
                                .as_deref()
                                .unwrap_or("4/4")
                                .to_string(),
                        ),
                    ],
                ),
            ),
            harp: chart.harmonica.display(),
            description: chart.metadata.as_ref().and_then(|m| m.description.clone()),
            chart_author: chart.metadata.as_ref().and_then(|m| {
                m.author.as_ref().map(|author| {
                    String::from(
                        loc.msg_args("gameplay-chart-author", &[("author", author.clone())]),
                    )
                })
            }),
        }
    }
}

/// The title alone, for the strip that stays up while notes are falling.
///
/// Everything else in [`SongInfo`] is deliberately absent here: a player
/// mid-phrase is not reading a description, and the column it used to occupy
/// is worth more as highway.
pub fn spawn_song_header(parent: &mut ChildSpawnerCommands, info: &SongInfo) {
    parent.spawn((
        Text::new(info.title.clone()),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::srgb(0.82, 0.84, 0.92)),
    ));
}

/// The whole block, for the two moments the player is not playing: the
/// countdown before the first note, and the pause menu.
pub fn spawn_song_details(parent: &mut ChildSpawnerCommands, info: &SongInfo) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            max_width: Val::Px(560.0),
            ..default()
        })
        .with_children(|col| {
            for (text, size, color) in [
                (Some(&info.title), 20.0, Color::WHITE),
                (Some(&info.meter), 15.0, Color::srgb(0.60, 0.65, 0.75)),
                (Some(&info.harp), 15.0, Color::srgb(0.45, 0.72, 0.55)),
                (
                    info.description.as_ref(),
                    15.0,
                    Color::srgb(0.50, 0.50, 0.55),
                ),
                (
                    info.chart_author.as_ref(),
                    15.0,
                    Color::srgb(0.40, 0.40, 0.45),
                ),
            ] {
                let Some(text) = text else { continue };
                col.spawn((
                    Text::new(text.clone()),
                    TextFont {
                        font_size: FontSize::Px(size),
                        ..default()
                    },
                    TextColor(color),
                ));
            }
        });
}
