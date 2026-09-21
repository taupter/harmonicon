// SPDX-License-Identifier: MIT

//! The 3D scene around the notes: camera, lighting, backdrop, the note
//! track and hit zone, the harmonica model and its groove, the hole glow, and
//! the camera restore on exit.

use super::*;

// ── Setup ─────────────────────────────────────────────────────────────────────

pub(super) fn setup_camera_3d(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 14.0, 24.0).looking_at(Vec3::new(0.0, 0.0, HIT_Z - 18.0), Vec3::Y),
        GameplayCamera3D,
        GameplayRoot,
        Name::new("Camera3d (gameplay 3D)"),
    ));
}

pub(super) fn setup_lighting(commands: &mut Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
            color: Color::srgb(1.0, 0.97, 0.90),
            ..default()
        },
        Transform::from_xyz(8.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        GameplayRoot,
    ));
    commands.spawn((
        AmbientLight {
            color: Color::srgb(0.15, 0.15, 0.22),
            brightness: 200.0,
            ..default()
        },
        GameplayRoot,
    ));
}

pub fn setup_background(commands: &mut Commands, background: Handle<Image>) {
    // Mesh and material are built inline with `asset_value`, so the scene adds
    // them via the `AssetServer` at spawn time — no `Assets` params to thread.
    commands.spawn_scene(bsn! {
        Mesh3d({asset_value(Rectangle::new(200.0, 140.0))})
        MeshMaterial3d::<StandardMaterial>({asset_value(StandardMaterial {
            base_color_texture: Some(background),
            unlit: true,
            cull_mode: None,
            ..default()
        })})
        Transform { translation: {Vec3::new(0.0, 14.0, FAR_Z - 2.0)} }
        GameplayRoot
    });
}

pub(super) fn create_note_track(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    total_width: f32,
    lane_width: f32,
    track_len: f32,
    center_x: f32,
    track_ctr_z: f32,
    holes: &[HoleConfig],
) {
    // Semi-translucent floor so notes dipping below the lane (downward bends)
    // stay visible. Mesh + material built inline via `asset_value`.
    commands.spawn_scene(bsn! {
        Mesh3d({asset_value(Cuboid::new(total_width, 0.05, track_len))})
        MeshMaterial3d::<StandardMaterial>({asset_value(StandardMaterial {
            base_color: Color::srgba(0.08, 0.08, 0.12, 0.5),
            alpha_mode: AlphaMode::Blend,
            metallic: 0.3,
            perceptual_roughness: 0.8,
            ..default()
        })})
        Transform { translation: {Vec3::new(center_x, LANE_Y - 0.025, track_ctr_z)} }
        GameplayRoot
    });

    // Alternating-lane shading is per-hole (a loop), so it stays imperative.
    for (i, hole) in holes.iter().enumerate() {
        if i % 2 == 1 {
            continue;
        }
        let shade_mat = materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.04),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        let shade_mesh = meshes.add(Cuboid::new(lane_width, 0.04, track_len));
        commands.spawn((
            Mesh3d(shade_mesh),
            MeshMaterial3d(shade_mat),
            Transform::from_xyz(hole.x, LANE_Y, track_ctr_z),
            GameplayRoot,
        ));
    }
}

pub(super) fn create_hit_zone(commands: &mut Commands, center_x: f32, total_width: f32) {
    commands.spawn_scene(bsn! {
        Mesh3d({asset_value(Cuboid::new(total_width, 0.06, 2.8))})
        MeshMaterial3d::<StandardMaterial>({asset_value(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 0.6, 0.15),
            emissive: LinearRgba::new(0.8, 0.8, 0.2, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })})
        Transform { translation: {Vec3::new(center_x, LANE_Y + 0.03, HIT_Z)} }
        GameplayRoot
    });
}

pub(super) fn spawn_harmonica_3d(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    model_name: &str,
    config: &HarmonicaModelConfig,
) {
    let [tx, ty, tz] = config.model_translation;

    // Everything that should "dance" lives under one parent. `groove_harmonica`
    // nudges this parent's Transform; children (model + holes) inherit the motion
    // so the hole overlays stay glued to the model face.
    commands
        .spawn((HarmonicaGroove, GameplayRoot))
        .with_children(|groove| {
            groove.spawn((
                WorldAssetRoot(
                    asset_server.load(format!("harmonicas/3d/{model_name}/harmonica.glb#Scene0")),
                ),
                Transform::from_xyz(tx, ty, tz)
                    .with_rotation(Quat::from_rotation_y(
                        config.model_rotation_y_deg.to_radians(),
                    ))
                    .with_scale(Vec3::splat(config.model_scale)),
            ));

            for (i, hole_cfg) in config.holes.iter().enumerate() {
                let hole = (i + 1) as u8;
                let hole_mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.10, 0.11, 0.15),
                    emissive: LinearRgba::new(0.0, 0.0, 0.0, 0.0),
                    metallic: 0.3,
                    perceptual_roughness: 0.6,
                    ..default()
                });
                let hole_mesh = meshes.add(Cuboid::new(hole_cfg.w, hole_cfg.h, hole_cfg.d));
                let mat_handle = hole_mat.clone();
                groove.spawn((
                    Mesh3d(hole_mesh),
                    MeshMaterial3d(hole_mat),
                    Transform::from_xyz(hole_cfg.x, hole_cfg.y, hole_cfg.z),
                    HoleCell(hole),
                    HoleMesh3D(mat_handle),
                ));
            }
        });
}

// ── Per-frame systems ─────────────────────────────────────────────────────────

/// Deterministic pseudo-random in -1..1 from an integer-ish input. The classic
/// `fract(sin(x) * big)` hash — repeatable per beat, so the groove is stable but
/// looks improvised.
pub(super) fn hash11(n: f32) -> f32 {
    let x = (n * 127.1).sin() * 43758.547;
    x.fract() * 2.0 - 1.0
}

/// Sways the harmonica a few millimeters in all directions, in time with the
/// song's tempo, so it looks like it's grooving to the music. The motion has a
/// blues shuffle: a triplet bounce, a backbeat accent, and per-beat randomness
/// so it never settles into a metronomic rocking-chair arc.
pub fn groove_harmonica(
    clock: Res<super::super::GameplayClock>,
    selected: Res<SelectedSong>,
    manifests: Res<Assets<SongManifest>>,
    mut groove: Query<&mut Transform, With<HarmonicaGroove>>,
) {
    use std::f32::consts::{PI, TAU};

    let Some(manifest) = manifests.get(&selected.0) else {
        return;
    };
    // Hold still during the countdown (clock is negative); only dance once the
    // music has started.
    if clock.get() < 0.0 {
        for mut tf in &mut groove {
            tf.translation = Vec3::ZERO;
            tf.rotation = Quat::IDENTITY;
        }
        return;
    }

    let bpm = manifest.chart.song.tempo_bpm.max(1.0);
    // Beats elapsed (fractional).
    let beat = (clock.get() / (60.0 / bpm as f64)) as f32;
    let bi = beat.floor();
    let frac = beat.fract();

    // Per-beat random accent, smoothly interpolated across the beat (smoothstep)
    // so each beat lands a little differently — the "improvised" blues feel.
    let s = frac * frac * (3.0 - 2.0 * frac);
    let accent = hash11(bi) + (hash11(bi + 1.0) - hash11(bi)) * s;

    // Backbeat emphasis on beats 2 & 4 (the blues snare hits), the off-beats get
    // a stronger kick than the downbeats.
    let backbeat = if (bi as i32).rem_euclid(2) == 1 {
        1.0
    } else {
        0.6
    };

    // Triplet shuffle: a strong hit on the beat plus a lighter swung hit on the
    // last triplet (the "and-a"). This is what gives the bounce its blues swing
    // instead of an even, sea-saw oscillation.
    let shuffle = (beat * TAU).sin() * 0.7 + (beat * TAU * 1.5).sin() * 0.3;
    let bob = shuffle * backbeat * (0.6 + 0.4 * accent);

    // Quasi-periodic sway/nod: layering sines at incommensurate (non-integer)
    // ratios means they never line up the same way twice, so the side-to-side and
    // fore/aft never repeat into a clean rocking arc. The accent nudges them too.
    let sway = (beat * PI).sin() * 0.6 + (beat * PI * 0.37).sin() * 0.25 + accent * 0.35;
    let nod = (beat * PI * 0.73 + PI * 0.25).cos() * 0.6 + (beat * TAU * 0.21).sin() * 0.4;

    for mut tf in &mut groove {
        tf.translation = Vec3::new(sway * 0.03, bob * 0.022, nod * 0.018);
        tf.rotation = Quat::from_rotation_z(sway * 0.018)
            * Quat::from_rotation_x(nod * 0.012)
            * Quat::from_rotation_y(accent * 0.012);
    }
}

pub fn update_holes_3d(
    time: Res<Time>,
    active: Res<ActivePitches>,
    valid_notes: Res<ValidHarpNotes>,
    targets: Res<ActiveTargets>,
    played: Res<PlayedHarp>,
    lesson: Option<Res<harmonicon_song::lessons::LessonContext>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cells: Query<(&HoleCell, &HoleMesh3D, &mut HoleState)>,
) {
    // The played harp's notes, as in `gameplay_2d::update_holes`.
    let Some(harp) = played.0.as_ref() else {
        return;
    };
    let dt = time.delta_secs();

    let attack = 1.0 - (-dt * 25.0_f32).exp();
    let decay = 1.0 - (-dt * 4.0_f32).exp();
    let harp_pitches = super::super::gameplay_2d::harp_pitches(&active, &valid_notes);

    for (cell, hole_mat, mut state) in &mut cells {
        let blow = harp.wind_direction_midi(cell.0, &Action::Blow);
        let draw = harp.wind_direction_midi(cell.0, &Action::Draw);
        let hint = if lesson.as_ref().is_some_and(|lesson| lesson.aural) {
            None
        } else {
            targets
                .0
                .iter()
                .find(|(h, _)| *h == cell.0)
                .map(|(_, b)| *b)
        };

        super::super::gameplay_2d::step_hole_glow(
            &mut state,
            blow,
            draw,
            hint,
            &harp_pitches,
            attack,
            decay,
        );
        let b = state.brightness;

        if let Some(mut mat) = materials.get_mut(&hole_mat.0) {
            if state.is_blow {
                mat.emissive =
                    LinearRgba::new(0.05 + 0.15 * b, 0.10 + 0.50 * b, 0.10 + 2.0 * b, 1.0);
                mat.base_color = Color::srgb(0.05 + 0.20 * b, 0.08 + 0.40 * b, 0.08 + 0.75 * b);
            } else {
                mat.emissive = LinearRgba::new(0.05 + 2.0 * b, 0.05 + 0.40 * b, 0.02, 1.0);
                mat.base_color =
                    Color::srgb(0.08 + 0.78 * b, 0.06 + 0.25 * b, (0.08 - 0.04 * b).max(0.0));
            }
        }
    }
}

/// Called on `OnExit(AppState::Playing)` — restores Camera2d to its normal
/// state so the 2D menu renders correctly after leaving 3D gameplay.
pub fn restore_camera(mut cameras: Query<(&mut Camera, &mut Transform), With<Camera2d>>) {
    for (mut cam, _) in &mut cameras {
        cam.order = 0;
        cam.clear_color = ClearColorConfig::Default;
    }
}
