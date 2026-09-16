// SPDX-License-Identifier: MIT

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui_render::prelude::MaterialNode;
use bevy::ui_widgets::Activate;
use bevy::ui_widgets::Button as WidgetButton;
use bevy_fluent::Localization;

use super::gameplay_2d::{note_anim_mode, note_techniques};
use super::note_tail_2d::{NoteTail2dMaterial, tail_params};
use harmonicon_core::chart::Modifier;
use harmonicon_platform::localization::LocalizationExt;

/// Whether the techniques legend body is collapsed, toggled by clicking its
/// header. Not reset on song load — like [`super::metronome_overlay::
/// MetronomeMuted`], a player's preference should outlive one song.
#[derive(Resource, Default)]
pub struct TechniqueLegendCollapsed(pub bool);

/// The column of technique rows, hidden/shown by [`TechniqueLegendCollapsed`].
#[derive(Component)]
struct TechniqueLegendBody;

/// The header's text, carrying the collapse/expand arrow.
#[derive(Component, Default, Clone)]
struct TechniqueLegendToggleLabel;

/// The techniques shown in the legend, paired with their label. Example
/// modifiers carry representative intensities so each preview animates
/// clearly; actual params/animation come from the same `note_techniques`/
/// `note_anim_mode`/`tail_params` the falling notes use, so the legend
/// can't drift from what the notes do.
fn legend_techniques() -> [(Modifier, &'static str); 6] {
    use harmonicon_core::chart::Modifier::*;
    [
        (
            Bend {
                semitones: -1.0,
                intensity: None,
            },
            "mod-bend",
        ),
        (
            Vibrato {
                oscillation_hz: 5.0,
                intensity: Some(0.9),
            },
            "mod-vibrato",
        ),
        (
            WahWah {
                oscillation_hz: 3.0,
                intensity: Some(0.9),
            },
            "mod-wah",
        ),
        (Overblow, "mod-overblow"),
        (Overdraw, "mod-overdraw"),
        (Slide, "mod-slide"),
    ]
}

fn modifier_kind(modifier: &Modifier) -> u8 {
    match modifier {
        Modifier::Bend { .. } => 0,
        Modifier::Vibrato { .. } => 1,
        Modifier::WahWah { .. } => 2,
        Modifier::Overblow => 3,
        Modifier::Overdraw => 4,
        Modifier::Slide => 5,
    }
}

/// Representative legend entries for only the techniques present in a chart.
/// Keeping this decision independent of rendering makes it impossible for a
/// plain-note song to spend permanent HUD space teaching five unrelated
/// symbols, and gives chromatic slide charts the entry the old fixed legend
/// omitted entirely.
fn used_legend_techniques(modifiers: &[Modifier]) -> Vec<(Modifier, &'static str)> {
    legend_techniques()
        .into_iter()
        .filter(|(candidate, _)| {
            modifiers
                .iter()
                .any(|used| modifier_kind(used) == modifier_kind(candidate))
        })
        .collect()
}

/// Builds one comet-tail material per technique for the legend previews. They are
/// regular `NoteTail2dMaterial`s, so `animate_note_tails` drives them in time with
/// everything else. A neutral colour is used on purpose — the *animation*, not the
/// colour, now tells the techniques apart.
pub fn build_legend_materials(
    materials: &mut Assets<NoteTail2dMaterial>,
    used_modifiers: &[Modifier],
) -> Vec<(Handle<NoteTail2dMaterial>, &'static str)> {
    // A short, fixed preview "note": enough length for the animations to read.
    const PREVIEW_H_PCT: f32 = 20.0;
    let color = Color::srgba(0.74, 0.82, 1.0, 0.95).to_linear();

    used_legend_techniques(used_modifiers)
        .into_iter()
        .enumerate()
        .map(|(i, (modifier, name))| {
            let slice = std::slice::from_ref(&modifier);
            let (vib, shift, wah) = note_techniques(Some(slice));
            let mode = note_anim_mode(Some(slice));
            let (mut params, mut wah_v) = tail_params(PREVIEW_H_PCT, vib, shift, wah);
            params.z = 0.0; // animation clock, driven by animate_note_tails
            wah_v.z = mode; // which technique animation
            wah_v.w = i as f32 * 1.3; // stagger the phases
            let handle = materials.add(NoteTail2dMaterial {
                color,
                params,
                wah: wah_v,
            });
            (handle, name)
        })
        .collect()
}

/// Arrow shown on the collapse/expand toggle, matched to `collapsed`.
fn toggle_arrow(collapsed: bool) -> &'static str {
    if collapsed { "\u{25B6}" } else { "\u{25BC}" }
}

/// The "▼ TECHNIQUES" toggle-header text for the given collapsed state —
/// shared by the initial `bsn!` placeholder and
/// [`update_technique_legend_visibility`] so the two can't drift apart.
fn technique_legend_toggle_text(loc: &Localization, collapsed: bool) -> String {
    loc.msg_args(
        "gameplay-techniques-toggle",
        &[("arrow", toggle_arrow(collapsed).to_string())],
    )
    .into()
}

/// Spawns the techniques legend: a small *animated tail* preview beside each
/// technique's name, so players learn to read a note by its motion, stacked
/// one per row under a clickable header that collapses/expands the list. Used
/// by both the 2D and 3D HUDs. `entries` come from [`build_legend_materials`].
pub fn spawn_modifier_legend(
    parent: &mut ChildSpawnerCommands,
    loc: &Localization,
    entries: &[(Handle<NoteTail2dMaterial>, &'static str)],
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn_empty().apply_scene(bsn! {
                WidgetButton
                TabIndex(0)
                Node { padding: {UiRect::ZERO} }
                BackgroundColor({Color::NONE})
                on(toggle_technique_legend)
                Children [
                    (
                        Text({technique_legend_toggle_text(loc, false)})
                        TextFont { font_size: {FontSize::Px(15.0)} }
                        TextColor({Color::srgb(0.55, 0.55, 0.62)})
                        TechniqueLegendToggleLabel
                        Pickable { should_block_lower: {false}, is_hoverable: {false} }
                    )
                ]
            });

            // One technique per row (icon left, name right), stacked vertically
            // instead of wrapping, so the legend's width never varies with how
            // many entries fit per line.
            col.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                TechniqueLegendBody,
            ))
            .with_children(|list| {
                for (handle, label_key) in entries {
                    list.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(5.0),
                        ..default()
                    })
                    .with_children(|row| {
                        // The live, animated comet tail for this technique.
                        row.spawn((
                            Node {
                                width: Val::Px(18.0),
                                height: Val::Px(38.0),
                                ..default()
                            },
                            MaterialNode(handle.clone()),
                        ));
                        row.spawn((
                            Text::new(String::from(loc.msg(label_key))),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.70, 0.72, 0.78)),
                        ));
                    });
                }
            });
        });
}

fn toggle_technique_legend(_: On<Activate>, mut collapsed: ResMut<TechniqueLegendCollapsed>) {
    collapsed.0 = !collapsed.0;
}

/// Mirrors [`TechniqueLegendCollapsed`] onto the body's visibility and the
/// header's arrow, written every frame (like `update_mute_label`) so a
/// freshly spawned legend — a new one is spawned per song — isn't stale.
fn update_technique_legend_visibility(
    collapsed: Res<TechniqueLegendCollapsed>,
    loc: Res<Localization>,
    mut bodies: Query<&mut Node, With<TechniqueLegendBody>>,
    mut labels: Query<&mut Text, With<TechniqueLegendToggleLabel>>,
) {
    for mut node in &mut bodies {
        node.display = if collapsed.0 {
            Display::None
        } else {
            Display::Flex
        };
    }
    for mut text in &mut labels {
        *text = Text::new(technique_legend_toggle_text(&loc, collapsed.0));
    }
}

pub struct ModifierLegendPlugin;

impl Plugin for ModifierLegendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TechniqueLegendCollapsed>()
            .add_systems(Update, update_technique_legend_visibility);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legend_covers_all_techniques() {
        assert_eq!(legend_techniques().len(), 6);
    }

    #[test]
    fn legend_names_match_the_techniques() {
        let names: Vec<&str> = legend_techniques().iter().map(|(_, n)| *n).collect();
        assert_eq!(
            names,
            [
                "mod-bend",
                "mod-vibrato",
                "mod-wah",
                "mod-overblow",
                "mod-overdraw",
                "mod-slide"
            ]
        );
    }

    #[test]
    fn legend_contains_only_techniques_the_chart_uses_in_canonical_order() {
        let used = [
            Modifier::Slide,
            Modifier::Vibrato {
                oscillation_hz: 4.0,
                intensity: None,
            },
            Modifier::Slide,
        ];
        let names: Vec<&str> = used_legend_techniques(&used)
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        assert_eq!(names, ["mod-vibrato", "mod-slide"]);
    }

    #[test]
    fn plain_notes_need_no_technique_legend() {
        assert!(used_legend_techniques(&[]).is_empty());
    }
}
