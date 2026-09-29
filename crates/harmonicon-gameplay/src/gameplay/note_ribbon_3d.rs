// SPDX-License-Identifier: MIT

use bevy::pbr::{Material, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// A 3D note: one flat ribbon on its lane, as long as the note lasts, with a
/// bright cap at the front edge and the technique drawn along it in world
/// units (see `note_ribbon_3d.wesl`). There is no separate head — the cap
/// is the attack.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct NoteRibbon3dMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    /// x = mode (0 plain, 1 bend, 2 vibrato, 3 wah, 4 pitch-up shift),
    /// y = oscillation cycles per world unit, z = signed lean at full bend,
    /// w = 1 when a vibrato rides on a bend — see
    /// `gameplay_3d::ribbon_technique`.
    #[uniform(1)]
    pub technique: Vec4,
    /// x = ribbon length and y = cap length (world units), z = animation
    /// time in seconds (refreshed each frame by `animate_note_ribbons_3d`),
    /// w = per-note phase.
    #[uniform(2)]
    pub shape: Vec4,
    /// Live hold progress once the note is hit — see
    /// `note_feedback::hold_uniform` for the layout. Zero until then.
    #[uniform(3)]
    pub hold: Vec4,
}

impl Material for NoteRibbon3dMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/note_ribbon_3d.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

pub struct NoteRibbon3dPlugin;

impl Plugin for NoteRibbon3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<NoteRibbon3dMaterial>::default());
    }
}
