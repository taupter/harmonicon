// SPDX-License-Identifier: MIT

//! 3D notes: building `SongNotes` for a chart, spawning each note's cube
//! head, tail ribbon and floating hole label in the `LOOKAHEAD` window,
//! scrolling and recycling them, and the per-frame tint / judged-note /
//! tail animation systems — the 3D twin of `gameplay_2d`'s note path.

use super::*;

/// `(note_w, head_depth, tail_len)` for `hole`/`duration` — everything
/// `spawn_visible_notes_3d` (at spawn) and `update_notes_3d` (every frame)
/// need beyond what's already on `ScheduledNote`. Recomputed on demand
/// rather than cached, since it's cheap: just the hole's configured width
/// and the note's own duration.
pub(super) fn note_dimensions(
    assets: &NoteRenderAssets3D,
    hole: u8,
    duration: f64,
) -> (f32, f32, f32) {
    let hole_cfg = assets.holes.get(hole.saturating_sub(1) as usize);
    let note_w = hole_cfg.map(|h| h.w).unwrap_or(LANE_WIDTH - LANE_GAP);
    let head_scale = note_w * assets.cfg.as_ref().map(|c| c.head_scale).unwrap_or(1.0);
    let head_depth = head_scale * 1.4;
    let tail_len = note_depth(duration);
    (note_w, head_depth, tail_len)
}

/// `hole_count` comes from the loaded chart's harmonica (10 for diatonic,
/// more for chromatic) — not a fixed constant, so lanes/notes land in the
/// right place regardless of harmonica type.
pub(super) fn lane_x(hole: u8, hole_count: u8) -> f32 {
    (hole as f32 - 1.0) * LANE_WIDTH - (hole_count as f32 * LANE_WIDTH) / 2.0 + LANE_WIDTH * 0.5
}

pub(super) fn note_depth(duration: f64) -> f32 {
    ((duration as f32 / LOOKAHEAD as f32) * LANE_DEPTH).clamp(0.4, 12.0)
}

/// Spawns each note as a 3D comet: an elongated cube head (from the theme's
/// glTF) tinted by blow/draw colour, trailing a flat ribbon that runs the
/// technique's animation via [`NoteTail3dMaterial`] — the 3D twin of the 2D
/// head+tail comet. Builds every note's score state (`SongNotes`) plus the
/// render config `spawn_visible_notes_3d` needs (`NoteRenderAssets3D`) — no
/// entities yet; notes spawn lazily in a `LOOKAHEAD` window around the
/// playhead, mirroring `gameplay_2d::spawn_visible_notes`.
pub(super) fn build_song_notes_3d(
    effective: &EffectiveHarmonica,
    chart: &HarpChart,
    head_mesh: Handle<Mesh>,
    cfg: NoteCube3dConfig,
    holes: Vec<HoleConfig>,
    adaptive: &AdaptiveDifficulty,
) -> (super::super::SongNotes, NoteRenderAssets3D) {
    let (notes, _) = super::super::build_scheduled_notes(effective, chart, adaptive);
    let hole_count = effective.harp_for(chart).hole_count();
    (
        super::super::SongNotes { notes, cursor: 0 },
        NoteRenderAssets3D {
            head_mesh: Some(head_mesh),
            cfg: Some(cfg),
            holes,
            hole_count,
        },
    )
}

/// Spawns 3D note visuals for any note newly within the `LOOKAHEAD` window.
/// Self-healing across a loop wrap, same as the 2D version: no persistent
/// spawn cursor, just "is this note's window open, and does it already have
/// a visual" recomputed each frame.
pub fn spawn_visible_notes_3d(
    mut commands: Commands,
    clock: Res<super::super::GameplayClock>,
    song_notes: Res<super::super::SongNotes>,
    render_assets: Res<NoteRenderAssets3D>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tail_materials: ResMut<Assets<NoteTail3dMaterial>>,
    existing: Query<&NoteVisual3D>,
    show_numbers: Res<ShowNoteNumbers>,
    theme: Res<LoadedTheme>,
    colorblind: Res<harmonicon_platform::settings::ColorblindPalette>,
    lesson: Option<Res<harmonicon_song::lessons::LessonContext>>,
) {
    if lesson.is_some_and(|lesson| lesson.aural) {
        return;
    }
    if render_assets.head_mesh.is_none() {
        return;
    }
    let colors = effective_note_colors(theme.note_colors(), colorblind.0);
    let elapsed = clock.get();
    let already_spawned: HashSet<usize> = existing.iter().map(|v| v.note_id).collect();
    for i in super::super::notes_needing_spawn(&song_notes.notes, &already_spawned, elapsed) {
        if note_has_left_view(&render_assets, &song_notes.notes[i], elapsed) {
            continue;
        }
        spawn_note_visual_3d(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut tail_materials,
            &render_assets,
            i,
            &song_notes.notes[i],
            show_numbers.0,
            colors,
        );
    }
}

/// A note's base (un-hit, un-missed) blow/draw appearance: `(r, g, b,
/// emissive_r, emissive_g, emissive_b)`. `r`/`g`/`b` come from `colors`
/// (the active theme's note colors, or the fixed colorblind-safe pair —
/// see `theme::effective_note_colors`); the emissive glow stays a fixed
/// per-direction accent regardless of palette, a secondary bloom layered
/// on top of the palette-driven base color. Shared by `spawn_note_visual_3d`
/// and `update_note_visuals_3d` so the two can't drift out of sync.
pub(super) fn note_base_appearance(
    colors: NoteColors,
    is_blow: bool,
) -> (f32, f32, f32, f32, f32, f32) {
    let c = if is_blow { colors.blow } else { colors.draw }.to_srgba();
    let (emit_r, emit_g, emit_b) = if is_blow {
        (0.1, 0.3, 1.2)
    } else {
        (1.2, 0.2, 0.05)
    };
    (c.red, c.green, c.blue, emit_r, emit_g, emit_b)
}

/// Spawns one note as a 3D comet: an elongated cube head (from the theme's
/// glTF) tinted by blow/draw colour, trailing a flat ribbon that runs the
/// technique's animation via [`NoteTail3dMaterial`] — the 3D twin of the 2D
/// head+tail comet. Positioned once here (holes don't move); `update_notes_3d`
/// drives the Z position every frame.
pub(super) fn spawn_note_visual_3d(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    tail_materials: &mut Assets<NoteTail3dMaterial>,
    assets: &NoteRenderAssets3D,
    note_id: usize,
    note: &ScheduledNote,
    show_numbers: bool,
    colors: NoteColors,
) {
    let head_mesh = assets.head_mesh.as_ref().expect("checked by caller");
    let cfg = assets.cfg.as_ref().expect("checked by caller");
    let (r, g, b, emit_r, emit_g, emit_b) = note_base_appearance(colors, note.is_blow);

    let hole_cfg = assets.holes.get(note.hole.saturating_sub(1) as usize);
    let note_x = hole_cfg
        .map(|h| h.x)
        .unwrap_or_else(|| lane_x(note.hole, assets.hole_count));
    let (note_w, head_depth, tail_len) = note_dimensions(assets, note.hole, note.duration);

    // Head: the elongated cube (1.4 units long in Z), tinted blow/draw.
    let head_scale = note_w * cfg.head_scale;
    let head_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(r, g, b),
        emissive: LinearRgba::new(emit_r, emit_g, emit_b, 1.0),
        ..default()
    });

    // Tail: a flat ribbon driven by the same technique animation as 2D.
    let (vib, shift, wah) = note_techniques(Some(&note.modifiers));
    let mode = note_anim_mode(Some(&note.modifiers));
    let (mut params, mut wah_v) = tail_params(20.0, vib, shift, wah);
    params.z = 0.0; // animation clock, set each frame
    wah_v.z = mode; // which technique animation
    wah_v.w = note_id as f32 * 1.7; // per-note phase
    let tail_mat = tail_materials.add(NoteTail3dMaterial {
        color: Color::srgba(r, g, b, 0.9).to_linear(),
        params,
        wah: wah_v,
        hold: Vec4::ZERO,
    });
    let tail_w = note_w * cfg.tail_width;
    let tail_mesh = meshes.add(Mesh::from(Plane3d::new(
        Vec3::Y,
        Vec2::new(tail_w * 0.5, tail_len * 0.5),
    )));

    let note_entity = commands
        .spawn((
            Transform::from_xyz(note_x, LANE_Y + NOTE_H * 0.5, FAR_Z),
            NoteVisual3D { note_id },
            JudgedState::default(),
            GameplayRoot,
        ))
        .with_children(|note_e| {
            // Cube head at the leading edge (parent origin).
            note_e.spawn((
                Mesh3d(head_mesh.clone()),
                MeshMaterial3d(head_mat),
                Transform::from_scale(Vec3::splat(head_scale)),
                NoteHead3d {
                    base_scale: head_scale,
                },
            ));
            // Tail ribbon trailing behind the head (−Z), flat over the lane.
            note_e.spawn((
                Mesh3d(tail_mesh),
                MeshMaterial3d(tail_mat),
                Transform::from_xyz(
                    0.0,
                    -NOTE_H * 0.5 + 0.02,
                    -(head_depth * 0.5 + tail_len * 0.5),
                ),
                NoteTail3d,
            ));
        })
        .id();

    // Hole-number label: a separate UI entity (see `NoteHoleLabel3D`'s doc
    // comment for why), positioned every frame by `update_note_hole_labels_3d`
    // — hidden until then, since it starts at the origin.
    if show_numbers {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                Visibility::Hidden,
                NoteHoleLabel3D {
                    target: note_entity,
                },
                GameplayRoot,
            ))
            .with_children(|l| {
                l.spawn((
                    // `+`/`-` for blow/draw — see the matching comment in
                    // `gameplay_2d::spawn_note_visual`.
                    Text::new(super::super::phrase_overlay::tab_label(
                        note.hole,
                        note.is_blow,
                        &[],
                    )),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    NoteHoleLabelText3D,
                ));
            });
    }
}

/// Offset (logical px) from a note's projected screen position to where its
/// hole-number label's top-left corner should land — up and to the left, so
/// the label sits just outside the note instead of covering it.
pub(super) const NOTE_LABEL_OFFSET: Vec2 = Vec2::new(-24.0, -20.0);

/// Converts a `Camera::world_to_viewport` result into the `Val::Px` a UI
/// `Node`'s `left`/`top` needs, offset to the label's anchor point.
/// `world_to_viewport` resolves through `logical_viewport_rect()`, the same
/// logical-window-pixel space `Val::Px` is in — except bevy_ui additionally
/// multiplies every `Val::Px` by [`UiScale`] before converting to physical
/// pixels, a multiplier the camera projection knows nothing about. Dividing
/// by `ui_scale` here cancels that back out, so the label lands under the
/// note regardless of the player's UI zoom level (`dialogs::ui_scale`).
pub(super) fn note_label_position(viewport_px: Vec2, ui_scale: f32) -> Vec2 {
    viewport_px / ui_scale + NOTE_LABEL_OFFSET
}

/// Positions each [`NoteHoleLabel3D`] over its target note's current screen
/// position, or hides it once the note is behind the camera, or despawns it
/// once the note itself is gone (scrolled past and recycled).
///
/// Reads the note's local `Transform`, not `GlobalTransform`:
/// `update_notes_3d` (earlier in the same `Update` chain) writes
/// `Transform.translation.z` every frame, but `GlobalTransform`
/// propagation only runs afterward in `PostUpdate` — reading it here would
/// always be one frame stale, the label trailing behind its note. Note
/// root entities have no transform parent, so the local `Transform`
/// already *is* world space; nothing to wait on.
pub fn update_note_hole_labels_3d(
    mut commands: Commands,
    camera: Query<(&Camera, &GlobalTransform), With<GameplayCamera3D>>,
    ui_scale: Res<UiScale>,
    notes: Query<&Transform, With<NoteVisual3D>>,
    mut labels: Query<(Entity, &NoteHoleLabel3D, &mut Node, &mut Visibility)>,
) {
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };

    for (entity, label, mut node, mut visibility) in &mut labels {
        let Ok(note_transform) = notes.get(label.target) else {
            commands.entity(entity).despawn();
            continue;
        };
        match camera.world_to_viewport(camera_transform, note_transform.translation) {
            Ok(viewport_px) => {
                let pos = note_label_position(viewport_px, ui_scale.0);
                node.left = Val::Px(pos.x);
                node.top = Val::Px(pos.y);
                // Guarded so change detection (and the visibility-propagation
                // it triggers) doesn't fire every frame for every label while
                // nothing about their visibility actually changed.
                if *visibility != Visibility::Visible {
                    *visibility = Visibility::Visible;
                }
            }
            Err(_) => {
                if *visibility != Visibility::Hidden {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}

pub(super) fn note_has_left_view(
    assets: &NoteRenderAssets3D,
    note: &ScheduledNote,
    elapsed: f64,
) -> bool {
    let (_, head_depth, tail_len) = note_dimensions(assets, note.hole, note.duration);
    let distance = (elapsed - note.time) as f32 / LOOKAHEAD as f32 * LANE_DEPTH;
    distance > head_depth + tail_len + 4.0
}

pub fn update_notes_3d(
    clock: Res<super::super::GameplayClock>,
    song_notes: Res<super::super::SongNotes>,
    render_assets: Res<NoteRenderAssets3D>,
    mut commands: Commands,
    mut notes: Query<(Entity, &NoteVisual3D, &mut Transform)>,
) {
    let elapsed = clock.get();
    for (entity, visual, mut tf) in &mut notes {
        let Some(note) = song_notes.notes.get(visual.note_id) else {
            continue;
        };
        let (_, head_depth, _) = note_dimensions(&render_assets, note.hole, note.duration);
        let remaining = (note.time - elapsed) as f32;
        // The head's front face lands on the hit line at the note's time.
        let z = HIT_Z - remaining / LOOKAHEAD as f32 * LANE_DEPTH - head_depth * 0.5;
        // Recycle once the whole comet (head + trailing tail) has passed the
        // hit zone. Score state lives independently in `SongNotes` now, so
        // this despawns unconditionally even while looping —
        // `spawn_visible_notes_3d` respawns it once the (rewound) clock
        // nears it again, with no state to lose.
        if note_has_left_view(&render_assets, note, elapsed) {
            commands.entity(entity).despawn();
            continue;
        }
        tf.translation.z = z;
    }
}

/// Head/emissive/tail appearance for a 3D note visual: a gold head while
/// hit — the tail ribbon keeps its base colour so the shader's credited-hold
/// fill can advance along it — dim red while missed, otherwise its base
/// blow/draw appearance ([`note_base_appearance`]). Pulled out of
/// `update_note_visuals_3d` so the tint decision is unit-testable without
/// spinning up rendering — mirrors [`gameplay_2d::note_tint`].
pub(super) fn note_tint_3d(
    hit: bool,
    missed: bool,
    is_blow: bool,
    colors: NoteColors,
) -> (Color, LinearRgba, LinearRgba) {
    if hit {
        let (r, g, b, ..) = note_base_appearance(colors, is_blow);
        (
            Color::srgb(1.0, 0.9, 0.3),
            LinearRgba::new(2.5, 2.0, 0.3, 1.0),
            Color::srgba(r, g, b, 0.9).to_linear(),
        )
    } else if missed {
        (
            Color::srgb(0.4, 0.12, 0.12),
            LinearRgba::new(0.2, 0.05, 0.05, 1.0),
            Color::srgba(0.5, 0.13, 0.13, 0.6).to_linear(),
        )
    } else {
        let (r, g, b, emit_r, emit_g, emit_b) = note_base_appearance(colors, is_blow);
        (
            Color::srgb(r, g, b),
            LinearRgba::new(emit_r, emit_g, emit_b, 1.0),
            Color::srgba(r, g, b, 0.9).to_linear(),
        )
    }
}

/// Tints a 3D note's cube head and tail ribbon when it is hit or missed —
/// gold on a hit, dim red on a miss — mirroring the 2D path, and restores
/// the base blow/draw appearance otherwise (see [`note_tint_3d`]).
/// `ScheduledNote` isn't an ECS component (score state lives in
/// `SongNotes`), so this re-syncs every currently-spawned note's tint each
/// frame rather than reacting to `Changed<ScheduledNote>` — cheap since only
/// a `LOOKAHEAD` window's worth of notes are ever spawned.
pub fn update_note_visuals_3d(
    song_notes: Res<super::super::SongNotes>,
    clock: Res<super::super::GameplayClock>,
    audio: Res<harmonicon_audio::AudioSettings>,
    pitch_filter: Res<super::super::HarmonicaPitchFilter>,
    active: Res<ActivePitches>,
    valid_notes: Res<ValidHarpNotes>,
    notes: Query<(&NoteVisual3D, &Children)>,
    heads: Query<&MeshMaterial3d<StandardMaterial>, With<NoteHead3d>>,
    tails: Query<&MeshMaterial3d<NoteTail3dMaterial>, With<NoteTail3d>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut tail_materials: ResMut<Assets<NoteTail3dMaterial>>,
    theme: Res<LoadedTheme>,
    colorblind: Res<harmonicon_platform::settings::ColorblindPalette>,
) {
    let colors = effective_note_colors(theme.note_colors(), colorblind.0);
    let judged = judged_instant(clock.get(), &audio, Some(&pitch_filter));
    let sounding = harp_pitches(&active, &valid_notes);
    for (visual, children) in &notes {
        let Some(note) = song_notes.notes.get(visual.note_id) else {
            continue;
        };
        let (base, emissive, tail_color) =
            note_tint_3d(note.hit, note.missed, note.is_blow, colors);
        let hold = hold_uniform(
            note,
            judged,
            note.expected_pitch.is_some_and(|m| sounding.contains(&m)),
            live_technique_status(&note.modifiers, &note.pitch_samples, &note.amp_samples),
        );
        for child in children {
            if let Ok(h) = heads.get(*child)
                && let Some(mut m) = std_materials.get_mut(&h.0)
            {
                m.base_color = base;
                m.emissive = emissive;
            }
            if let Ok(h) = tails.get(*child)
                && let Some(mut m) = tail_materials.get_mut(&h.0)
            {
                m.color = tail_color;
                m.hold = hold;
            }
        }
    }
}

/// The 3D twin of `gameplay_2d::animate_judged_notes`: pops the cube head
/// on a hit, shrinks it on a miss, and stamps the floating hole label with
/// a check or cross — once per judgment, via [`JudgedState`], and undone
/// when an A–B loop clears the note.
pub fn animate_judged_notes_3d(
    mut commands: Commands,
    song_notes: Res<super::super::SongNotes>,
    clock: Res<super::super::GameplayClock>,
    reduced_motion: Res<harmonicon_platform::settings::ReducedMotion>,
    mut notes: Query<(
        Entity,
        &NoteVisual3D,
        &mut JudgedState,
        Option<&Judged>,
        &Children,
    )>,
    mut heads: Query<(&NoteHead3d, &mut Transform)>,
    labels: Query<(&NoteHoleLabel3D, &Children)>,
    mut label_texts: Query<&mut Text, With<NoteHoleLabelText3D>>,
) {
    let now = clock.get();
    for (entity, visual, mut state, judged, children) in &mut notes {
        let Some(note) = song_notes.notes.get(visual.note_id) else {
            continue;
        };
        let current = judged_now(note);
        let transitioned = current != state.0;
        let judged = if transitioned {
            state.0 = current;
            match current {
                Some(hit) => {
                    let j = Judged { hit, at: now };
                    commands.entity(entity).insert(j);
                    Some(j)
                }
                None => {
                    commands.entity(entity).remove::<Judged>();
                    None
                }
            }
        } else {
            judged.copied()
        };
        let scale = judged.map_or(1.0, |j| {
            judged_scale(j.hit, (now - j.at) as f32, reduced_motion.0)
        });
        for child in children {
            if let Ok((head, mut transform)) = heads.get_mut(*child) {
                let wanted = Vec3::splat(head.base_scale * scale);
                if transform.scale != wanted {
                    transform.scale = wanted;
                }
            }
        }
        if !transitioned {
            continue;
        }
        let wanted = match current {
            Some(hit) => judged_stamp(hit).to_string(),
            None => super::super::phrase_overlay::tab_label(note.hole, note.is_blow, &[]),
        };
        for (label, label_children) in &labels {
            if label.target != entity {
                continue;
            }
            for grandchild in label_children {
                if let Ok(mut text) = label_texts.get_mut(*grandchild) {
                    text.0 = wanted.clone();
                }
            }
        }
    }
}

/// Drives every 3D tail's animation clock (`params.z`) from the gameplay clock,
/// so the ribbons flow in time with the song and freeze on pause.
pub fn animate_note_tails_3d(
    clock: Res<super::super::GameplayClock>,
    reduced_motion: Res<harmonicon_platform::settings::ReducedMotion>,
    mut materials: ResMut<Assets<NoteTail3dMaterial>>,
) {
    if reduced_motion.0 {
        return;
    }
    let t = clock.get() as f32;
    for (_, material) in materials.iter_mut() {
        material.params.z = t;
    }
}
