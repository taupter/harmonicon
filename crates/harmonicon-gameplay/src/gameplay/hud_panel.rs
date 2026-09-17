// SPDX-License-Identifier: MIT

//! The side panel both scored modes carry: song title, the live phrase
//! banner and tab ribbon, the metronome, and the technique legend.
//!
//! Everything here is *live* material — it changes as the song plays — which
//! is what distinguishes it from `song_info::spawn_song_details`, the static
//! block that moved to the countdown and the pause menu.
//!
//! **One spawner, one order, one side.** 2D and 3D built this column
//! separately and identically except for where it sat (right vs. top-left)
//! and where the blow/draw key went, which is how they drifted into looking
//! like different games. A change to the panel is now a change to both; what
//! stays mode-specific is the lane surface and the note visuals, per
//! `docs/gameplay_improvement_plan.md`'s implementation boundaries.

use bevy::prelude::*;
use bevy_fluent::Localization;

use harmonicon_core::chart::Modifier;

use super::highway_2d::spawn_blow_draw_legend;
use super::metronome_overlay::spawn_metronome;
use super::modifier_legend::spawn_modifier_legend;
use super::note_tail_2d::NoteTail2dMaterial;
use super::phrase_overlay::{spawn_phrase_banner, spawn_tab_ribbon};
use super::song_info::{SongInfo, spawn_song_header};

/// What the panel needs to build itself. A struct rather than eight
/// positional arguments, because the two call sites are in different files
/// and a silently-swapped `bpm`/`beats_per_bar` pair would be invisible.
pub(super) struct HudPanel<'a> {
    pub song_info: &'a SongInfo,
    pub loc: &'a Localization,
    pub beats_per_bar: usize,
    pub bpm: f32,
    /// Empty when the chart uses no techniques — the legend is then omitted
    /// rather than shown empty.
    pub legend_materials: &'a [(Handle<NoteTail2dMaterial>, &'static str)],
    /// 2D prints the blow/draw key under its hole strip, where the colours
    /// it explains actually are. 3D has no hole strip, so it asks for the
    /// key inside the panel instead. The one honest difference between the
    /// two, and it follows from the lane surface rather than from drift.
    pub blow_draw_legend: bool,
}

pub(super) fn spawn_hud_panel(parent: &mut ChildSpawnerCommands, panel: HudPanel<'_>) {
    // Title only while notes are falling; key, harp, description and author
    // are read material and live where there is time to read them.
    parent.spawn(Node::default()).with_children(|col| {
        spawn_song_header(col, panel.song_info);
    });

    // Both driven per-frame by `phrase_overlay`.
    spawn_phrase_banner(parent);
    spawn_tab_ribbon(parent);

    if panel.blow_draw_legend {
        spawn_blow_draw_legend(parent, panel.loc, 12.0, 4.0);
    }

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|metro| {
            spawn_metronome(metro, panel.loc, panel.beats_per_bar, panel.bpm);
        });

    if !panel.legend_materials.is_empty() {
        spawn_modifier_legend(parent, panel.loc, panel.legend_materials);
    }
}

/// Every technique a chart actually uses, in chart order, for
/// [`modifier_legend::build_legend_materials`].
///
/// Shared so the legend can't list one set of techniques in 2D and another
/// in 3D — both modes flattened this out of `chart.track` themselves, and a
/// legend that disagrees with the notes is worse than no legend.
///
/// [`modifier_legend::build_legend_materials`]: super::modifier_legend::build_legend_materials
pub(super) fn used_modifiers(chart: &harmonicon_core::chart::HarpChart) -> Vec<Modifier> {
    chart
        .track
        .iter()
        .flat_map(|item| &item.events)
        .flat_map(|event| event.modifiers.as_deref().unwrap_or_default())
        .cloned()
        .collect()
}
