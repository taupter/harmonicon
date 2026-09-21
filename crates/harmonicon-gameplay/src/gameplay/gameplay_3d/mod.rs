// SPDX-License-Identifier: MIT

use std::collections::HashSet;

use bevy::prelude::*;
use harmonicon_core::chart::{Action, HarpChart};

use harmonicon_app::app::{EffectiveHarmonica, SelectedSong};
use harmonicon_platform::assets_management::{
    HarmonicaModelConfig, HoleConfig, SelectedHarmonicaModel, SelectedNoteTheme3d, ShowNoteNumbers,
};
use harmonicon_platform::theme::{HUD_PANEL_BG, LoadedTheme, NoteColors, effective_note_colors};
use harmonicon_song::song::NoteCube3dConfig;
use harmonicon_song::song::SongManifest;
use harmonicon_ui::music_score::{self, BravuraFont};

use super::adaptive_difficulty::AdaptiveDifficulty;
use super::countdown_overlay::spawn_countdown;
use super::gameplay_2d::{harp_pitches, note_anim_mode, note_techniques};
use super::hud_panel::{HudPanel, spawn_hud_panel, used_modifiers};
use super::judge::{judged_instant, live_technique_status};
use super::modifier_legend::build_legend_materials;
use super::note_feedback::{
    Judged, JudgedState, hold_uniform, judged_now, judged_scale, judged_stamp,
};
use super::note_tail_2d::{NoteTail2dMaterial, tail_params};
use super::note_tail_3d::NoteTail3dMaterial;
use super::song_progress_overlay::{BAR_HEIGHT, NoteMarker, spawn_song_progress};
use super::{
    ActivePitches, ActiveTargets, COUNTDOWN, GameplayRoot, HoleCell, HoleState, LOOKAHEAD,
    MusicStarted, PlayedHarp, ScheduledNote, ScoreReadoutAnchor, SongInfo, ValidHarpNotes,
    spawn_score_readout,
};
use harmonicon_platform::localization::Localization;

// ── 3D layout constants ───────────────────────────────────────────────────────

const LANE_WIDTH: f32 = 1.0;
const LANE_GAP: f32 = 0.06;
const LANE_DEPTH: f32 = 60.0;
const HIT_Z: f32 = 6.0;
/// Where the lane's hit plane lands on screen, as a fraction of window height
/// measured up from the bottom. The camera is fixed
/// (`Transform::from_xyz(0.0, 14.0, 24.0)` looking at the lane), so this is a
/// constant of that camera rather than something worth projecting per frame —
/// but it has to be re-measured if the camera ever moves.
const HIT_PLANE_BOTTOM_PCT: f32 = 24.0;
const FAR_Z: f32 = HIT_Z - LANE_DEPTH; // -54
const LANE_Y: f32 = 1.6;
const NOTE_H: f32 = 0.18;
const HARP_Z: f32 = HIT_Z + 2.2;

// ── 3D-only marker components ─────────────────────────────────────────────────

#[derive(Component)]
pub struct GameplayCamera3D;

#[derive(Component)]
#[require(Transform, Visibility)]
pub(super) struct NoteVisual3D {
    /// Index into `SongNotes::notes` — see the doc comment on the 2D
    /// `NoteVisual`, which this mirrors. `head_depth`/`tail_len` are cheap to
    /// recompute on demand from `NoteRenderAssets3D` + the note's own
    /// `hole`/`duration`, so there's nothing else this needs to carry.
    pub(super) note_id: usize,
}

/// A hole-number label tracking a 3D note (`ShowNoteNumbers` on). 3D notes
/// are opaque meshes with nowhere to put a number *on* them, so this is a
/// separate UI `Text` entity, positioned every frame by projecting
/// `target`'s world position through the gameplay camera
/// (`update_note_hole_labels_3d`) rather than living in the note's own
/// entity hierarchy — UI layout doesn't propagate through 3D `Transform`
/// parents. Despawns itself once `target` no longer exists (scrolled past
/// and recycled), so nothing needs to reach back into it from
/// `update_notes_3d`.
#[derive(Component)]
pub(super) struct NoteHoleLabel3D {
    target: Entity,
}

/// Chart-level (not per-note) 3D rendering config `spawn_visible_notes_3d`
/// needs once a note's `LOOKAHEAD` window arrives — set once at song load.
#[derive(Resource, Default)]
pub(super) struct NoteRenderAssets3D {
    head_mesh: Option<Handle<Mesh>>,
    cfg: Option<NoteCube3dConfig>,
    /// Per-hole x-position/width from the harmonica model's `holes.json`
    /// (falls back to `lane_x`/an even width when a hole has no entry).
    holes: Vec<HoleConfig>,
    hole_count: u8,
}

/// The cube head of a 3D note (child). Tinted gold/red on hit/miss and
/// scaled by `animate_judged_notes_3d` the moment the note is judged —
/// `base_scale` is what the pop/shrink multiplies, since the `Transform`
/// itself is overwritten each frame.
#[derive(Component)]
pub(super) struct NoteHead3d {
    base_scale: f32,
}

/// The tab text inside a [`NoteHoleLabel3D`]. Replaced by a check or cross
/// once the note is judged, as the 2D head label is.
#[derive(Component)]
pub(super) struct NoteHoleLabelText3D;

/// The animated tail ribbon of a 3D note (child). Tinted gold/red on hit/miss.
#[derive(Component)]
pub(super) struct NoteTail3d;

#[derive(Component)]
pub(super) struct HoleMesh3D(Handle<StandardMaterial>);

/// Parent entity holding the GLB model and its hole overlays. Animated by
/// `groove_harmonica` so the whole harmonica bobs in time with the music.
#[derive(Component)]
#[require(Transform, Visibility)]
pub(super) struct HarmonicaGroove;

// ── Harmonica model config ────────────────────────────────────────────────────

/// The fallback layout when a model has no `holes.json`: holes evenly
/// spaced across the lanes at the harmonica's resting position, sized to
/// the chart's actual hole count. (No bundled 3D model ships a chromatic
/// `holes.json`, so a chromatic chart's *note lanes* line up correctly
/// even though the harmonica prop still renders as whichever diatonic
/// model is selected — that needs a matching 3D asset, not just code.)
fn default_model_layout(hole_count: u8) -> HarmonicaModelConfig {
    HarmonicaModelConfig {
        model_translation: [0.0, LANE_Y + 0.45, HARP_Z],
        model_rotation_y_deg: 0.0,
        model_scale: 1.0,
        holes: (1u8..=hole_count)
            .map(|hole| HoleConfig {
                x: lane_x(hole, hole_count),
                y: LANE_Y + 0.9 + 0.10,
                z: HARP_Z,
                w: LANE_WIDTH - LANE_GAP - 0.08,
                h: 0.20,
                d: 0.90,
            })
            .collect(),
    }
}

fn load_model_config(model_name: &str, hole_count: u8) -> HarmonicaModelConfig {
    let path = format!("assets/harmonicas/3d/{model_name}/holes.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| {
            warn!("No holes.json for model '{model_name}', using default layout");
            default_model_layout(hole_count)
        })
}

/// Note-building state bundled into one `SystemParam` so `setup` stays under
/// Bevy's function-system parameter arity limit — plain individual params
/// would put it one over once `AdaptiveDifficulty` joined the list.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct NoteBuildState<'w> {
    valid_notes: ResMut<'w, ValidHarpNotes>,
    played_harp: ResMut<'w, PlayedHarp>,
    song_notes: ResMut<'w, super::SongNotes>,
    render_assets: ResMut<'w, NoteRenderAssets3D>,
    adaptive: Res<'w, AdaptiveDifficulty>,
}

/// Display/theming context bundled into one `SystemParam`, same reason as
/// [`NoteBuildState`] — `setup` was one param away from Bevy's arity limit
/// once `CompactLayout` joined the list. Unrelated to note-building, so a
/// separate bundle rather than folding into that one.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct HudContext<'w> {
    loc: Res<'w, Localization>,
    song_info: Res<'w, SongInfo>,
    bravura: Option<Res<'w, BravuraFont>>,
    compact: Res<'w, harmonicon_platform::responsive::CompactLayout>,
}

pub fn setup(
    effective: Res<EffectiveHarmonica>,
    mut commands: Commands,
    selected: Res<SelectedSong>,
    manifests: Res<Assets<SongManifest>>,
    mut clock: ResMut<super::GameplayClock>,
    mut music_started: ResMut<MusicStarted>,
    mut note_build: NoteBuildState,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    selected_model: Res<SelectedHarmonicaModel>,
    shape_materials: ResMut<Assets<NoteTail2dMaterial>>,
    note_theme: Res<SelectedNoteTheme3d>,
    mut cameras: Query<(&mut Camera, &mut Transform), With<Camera2d>>,
    hud: HudContext,
    lesson: Option<Res<harmonicon_song::lessons::LessonContext>>,
) {
    let compact = hud.compact.0;
    let Some(manifest): Option<&SongManifest> = manifests.get(&selected.0) else {
        error!("SongManifest not ready when entering Playing (3D) state");
        return;
    };
    clock.set_free(-COUNTDOWN);
    music_started.0 = false;
    (*note_build.valid_notes, *note_build.played_harp) =
        ValidHarpNotes::for_played_harp(&effective, &manifest.chart);

    for (mut cam, _) in &mut cameras {
        cam.order = 1;
        cam.clear_color = ClearColorConfig::None;
    }

    let chart = &manifest.chart;
    // The instrument on screen is the one the player is holding, as for 2D.
    let played = effective.harp_for(chart);
    let model_cfg = load_model_config(&selected_model.0, played.hole_count());

    setup_camera_3d(&mut commands);
    setup_lighting(&mut commands);
    setup_background(&mut commands, manifest.background.clone());

    let holes = &model_cfg.holes;
    let left_edge = holes.first().map(|h| h.x - h.w * 0.5).unwrap_or(-5.0);
    let right_edge = holes.last().map(|h| h.x + h.w * 0.5).unwrap_or(5.0);
    let total_width = right_edge - left_edge;
    let center_x = (left_edge + right_edge) * 0.5;
    let lane_width = total_width / holes.len() as f32;

    let track_end_z = holes.first().map(|h| h.z).unwrap_or(HARP_Z);
    let track_len = track_end_z - FAR_Z;
    let track_ctr_z = FAR_Z + track_len * 0.5;

    create_note_track(
        &mut commands,
        &mut meshes,
        &mut materials,
        total_width,
        lane_width,
        track_len,
        center_x,
        track_ctr_z,
        holes,
    );

    create_hit_zone(&mut commands, center_x, total_width);

    // Comet head mesh + 3D tail layout: loaded here — on entering the 3D game —
    // from the song's own GLB if it ships a `3d/` folder, else the selected
    // theme's default. The handle lives only on the note entities, so it frees
    // when they despawn on leaving the song.
    let head_mesh: Handle<Mesh> = match &manifest.assets_3d {
        Some(path) => asset_server.load(path.clone().with_label("Mesh0/Primitive0")),
        None => asset_server.load(format!("notes/3d/{}.glb#Mesh0/Primitive0", note_theme.0)),
    };
    let note_cfg = manifest.assets_3d_config.clone();
    let (notes, assets) = build_song_notes_3d(
        &effective,
        chart,
        head_mesh,
        note_cfg,
        holes.clone(),
        &note_build.adaptive,
    );
    *note_build.song_notes = notes;
    *note_build.render_assets = assets;
    spawn_harmonica_3d(
        &mut commands,
        &mut meshes,
        &mut materials,
        &asset_server,
        &selected_model.0,
        &model_cfg,
    );

    // The meter's own beat count, for the HUD's beat dots — from the one
    // reading of the chart's meter gameplay has (`bars::chart_meter`).
    let beats_per_bar = usize::from(super::bars::chart_meter(chart).numerator.max(1));
    spawn_hud_overlay(
        &mut commands,
        chart,
        chart.song.tempo_bpm,
        beats_per_bar,
        shape_materials,
        &hud.loc,
        &hud.song_info,
        compact,
    );
    let aural = lesson.is_some_and(|lesson| lesson.aural);
    let note_markers: Vec<NoteMarker> = if aural {
        Vec::new()
    } else {
        note_build
            .song_notes
            .notes
            .iter()
            .map(|n| NoteMarker {
                time: n.time,
                duration: n.duration,
                hole: n.hole,
                is_blow: n.is_blow,
            })
            .collect()
    };
    spawn_song_progress(
        &mut commands,
        &manifest.waveform,
        manifest.music_duration_secs,
        &note_markers,
        played.hole_count(),
        &note_build.adaptive.sections,
        &note_build.adaptive.learned,
    );
    if !aural
        && !compact
        && let Some(bravura) = &hud.bravura
    {
        super::gameplay_2d::spawn_gameplay_music_score(&mut commands, bravura);
    }
    let harp_hint =
        super::song_info::harp_banner_text(played, &effective.song_key_for(chart), &hud.loc);
    spawn_countdown(
        &mut commands,
        &hud.loc,
        Some(&harp_hint),
        Some(&hud.song_info),
    );
}

fn spawn_hud_overlay(
    commands: &mut Commands,
    chart: &harmonicon_core::chart::HarpChart,
    bpm: f32,
    beats_per_bar: usize,
    mut shape_materials: ResMut<Assets<NoteTail2dMaterial>>,
    loc: &Localization,
    song_info: &SongInfo,
    compact: bool,
) {
    // The same panel 2D carries, on the same side of the screen. It used to
    // sit top-left here and right in 2D, with the same contents in a
    // different order — the drift `hud_panel` exists to stop. All of it is
    // supplementary, so compact mode skips the panel outright rather than
    // trimming it piecemeal.
    if !compact {
        let legend_materials = build_legend_materials(&mut shape_materials, &used_modifiers(chart));
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    // Below the song-progress bar (`BAR_HEIGHT`, pinned at the
                    // very top across the full width and always painted above
                    // the HUD — see `BAR_Z_INDEX`) so its text is never covered.
                    top: Val::Px(8.0 + BAR_HEIGHT + music_score::PANEL_HEIGHT),
                    right: Val::Px(8.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    // Fixed so the panel doesn't grow or shrink with the
                    // current song's title length — long text wraps instead.
                    max_width: Val::Px(420.0),
                    ..default()
                },
                // `HUD_PANEL_BG`, not another hand-tuned near-black: 2D
                // darkens the whole screen behind its panel, but here the
                // song's own artwork shows through, and a 55%-black wash
                // leaves the technique legend unreadable over a bright one.
                BackgroundColor(HUD_PANEL_BG),
                GlobalZIndex(1),
                GameplayRoot,
            ))
            .with_children(|panel| {
                spawn_hud_panel(
                    panel,
                    HudPanel {
                        song_info,
                        loc,
                        beats_per_bar,
                        bpm,
                        legend_materials: &legend_materials,
                        // No hole strip here to print the key under, unlike
                        // 2D — so it goes in the panel.
                        blow_draw_legend: true,
                    },
                );
            });
    }

    // Score/combo/judgment, at the height of the lane's own hit plane rather
    // than in a screen corner. The 3D hit zone is a mesh at `HIT_Z`, so its
    // screen position comes from the (fixed) camera rather than from layout —
    // hence a measured fraction of the window rather than a UI anchor, unlike
    // 2D where the highway node's own bottom is the hit line.
    let readout_root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(1),
            GameplayRoot,
            Pickable::IGNORE,
        ))
        .id();
    spawn_score_readout(
        commands,
        readout_root,
        ScoreReadoutAnchor {
            left: Val::Percent(0.0),
            width: Val::Percent(100.0),
            bottom: Val::Percent(HIT_PLANE_BOTTOM_PCT),
        },
    );
    // Same anchor for the wait-for-note card, a little above the hit plane.
    super::wait_freeze_overlay::spawn_wait_freeze_prompt(
        commands,
        readout_root,
        Val::Percent(HIT_PLANE_BOTTOM_PCT + 8.0),
    );
}

mod notes;
mod scene;
#[cfg(test)]
mod tests;

pub use notes::*;
pub use scene::*;
