// SPDX-License-Identifier: MIT

//! Collapse, compaction, viewport, and neighboring-unit transitions.

use std::collections::{HashMap, HashSet};

use bevy::a11y::AccessibilityNode;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, InteractionDisabled, ScrollPosition, UiTransform, Val2};
use bevy::ui_widgets::Button as WidgetButton;

use harmonicon_menu::menu::MenuPage;
use harmonicon_platform::settings::ReducedMotion;

use super::{
    ClusterMember, Endpoint, LayoutOwner, MovingEdge, UnitButton, UnitChevron, set_edge_geometry,
};

const TRANSITION_SECONDS: f32 = 0.22;

/// How far a collapse or slide transition advances this frame, as a fraction
/// of the whole. Under Reduced Motion it is the whole transition, so every
/// unit jumps to its end state on the first frame.
pub(super) fn transition_step(delta_secs: f32, reduced_motion: bool) -> f32 {
    if reduced_motion {
        1.0
    } else {
        delta_secs / TRANSITION_SECONDS
    }
}

#[derive(Resource, Default)]
pub(crate) struct CollapsedUnits(pub(super) HashSet<String>);

#[derive(Resource, Default)]
pub(crate) struct UnitExpansions(pub(super) HashMap<String, f32>);

#[derive(Resource, Default)]
pub(crate) struct PendingCompaction(pub(super) HashSet<String>);

/// Keeps a toggled unit at the same screen position across the rebuild its
/// toggle causes. `canvas_x` is filled in by the rebuild, and cleared once
/// [`restore_viewport_anchor`] has scrolled to it.
#[derive(Resource, Default)]
pub(crate) struct PendingViewportAnchor {
    pub(super) unit_id: Option<String>,
    pub(super) screen_x: f32,
    pub(super) canvas_x: Option<f32>,
    /// The horizontal scroll the rebuild started from. Slides are set up in
    /// canvas coordinates against it, so moving the scroll has to move them
    /// by the same amount (see [`shift_slides`]).
    pub(super) scroll_x_before: f32,
}

#[derive(Resource, Default)]
pub(crate) struct PreviousUnitPositions(pub(super) HashMap<String, f32>);

#[derive(Clone, Copy, Debug)]
pub(super) struct UnitSlide {
    pub(super) from_px: f32,
    pub(super) amount: f32,
}

#[derive(Resource, Default)]
pub(crate) struct UnitSlides(pub(super) HashMap<String, UnitSlide>);

#[derive(Component)]
pub(crate) struct LessonTreeScroller;

/// Last viewport position, retained while the reader or gameplay page owns
/// the screen. The tree itself is despawned on every page change.
#[derive(Resource, Default)]
pub(crate) struct LessonTreeViewport(pub(super) Vec2);

/// A lesson requested by the header locator. Its canvas position is filled
/// during tree construction, after a collapsed unit has been expanded and
/// the compact layout has consequently changed.
#[derive(Resource, Default)]
pub(crate) struct PendingLessonFocus {
    pub(super) lesson_id: Option<String>,
    pub(super) canvas_position: Option<Vec2>,
}

pub(crate) fn focus_pending_lesson(
    mut pending: ResMut<PendingLessonFocus>,
    mut scroller: Query<(&mut ScrollPosition, &ComputedNode), With<LessonTreeScroller>>,
) {
    let Some(target) = pending.canvas_position else {
        return;
    };
    let Some((mut position, computed)) = scroller.iter_mut().next() else {
        return;
    };
    let scale = computed.inverse_scale_factor;
    let viewport = computed.size() * scale;
    let content = computed.content_size() * scale;
    if viewport.min_element() <= 0.0 || content.min_element() <= 0.0 {
        return;
    }

    position.0 = centred_scroll(target, viewport, content);
    pending.lesson_id = None;
    pending.canvas_position = None;
}

pub(super) fn centred_scroll(target: Vec2, viewport: Vec2, content: Vec2) -> Vec2 {
    (target - viewport / 2.0).clamp(Vec2::ZERO, (content - viewport).max(Vec2::ZERO))
}

pub(crate) fn remember_viewport(
    scroller: Query<&ScrollPosition, With<LessonTreeScroller>>,
    mut saved: ResMut<LessonTreeViewport>,
) {
    let Some(position) = scroller.iter().next() else {
        return;
    };
    if saved.0 != position.0 {
        saved.0 = position.0;
    }
}

pub(crate) fn restore_viewport_anchor(
    mut anchor: ResMut<PendingViewportAnchor>,
    mut slides: ResMut<UnitSlides>,
    units: Res<PreviousUnitPositions>,
    mut scroller: Query<(&mut ScrollPosition, &ComputedNode), With<LessonTreeScroller>>,
) {
    let Some(canvas_x) = anchor.canvas_x else {
        return;
    };
    let Some((mut position, computed)) = scroller.iter_mut().next() else {
        return;
    };
    if computed.size().x <= 0.0 || computed.content_size().x <= 0.0 {
        return;
    }

    position.x = anchored_scroll(
        canvas_x,
        anchor.screen_x,
        computed.size().x,
        computed.content_size().x,
    );
    shift_slides(
        &mut slides.0,
        units.0.keys(),
        position.x - anchor.scroll_x_before,
    );
    anchor.unit_id = None;
    anchor.canvas_x = None;
}

/// Moves every unit's slide by `scroll_shift`, the distance the viewport
/// just scrolled, so the scroll change itself moves nothing on screen and
/// each unit glides from where it was drawn.
///
/// A rebuild sets slides up in canvas coordinates, which is only right
/// while the scroll is unchanged. A unit whose canvas position did not
/// change has no slide at all, yet the anchor's scroll still moves it on
/// screen, so it gets one here. The anchored unit's own shift cancels its
/// canvas move, and a slide that ends up negligible is dropped.
pub(super) fn shift_slides<'a>(
    slides: &mut HashMap<String, UnitSlide>,
    units: impl IntoIterator<Item = &'a String>,
    scroll_shift: f32,
) {
    if scroll_shift == 0.0 {
        return;
    }
    for id in units {
        let slide = slides.entry(id.clone()).or_insert(UnitSlide {
            from_px: 0.0,
            amount: 0.0,
        });
        slide.from_px += scroll_shift;
    }
    slides.retain(|_, slide| slide.from_px.abs() > 0.5);
}

pub(super) fn anchored_scroll(
    canvas_x: f32,
    screen_x: f32,
    viewport_width: f32,
    content_width: f32,
) -> f32 {
    let max_scroll = (content_width - viewport_width).max(0.0);
    (canvas_x - screen_x).clamp(0.0, max_scroll)
}

pub(crate) fn animate_unit_expansion(
    mut commands: Commands,
    time: Res<Time>,
    reduced_motion: Res<ReducedMotion>,
    collapsed: Res<CollapsedUnits>,
    mut expansions: ResMut<UnitExpansions>,
    mut members: Query<(&ClusterMember, &mut UiTransform, &mut Visibility)>,
    mut chevrons: Query<(&UnitChevron, &mut Text)>,
    mut unit_buttons: Query<(&UnitButton, &mut AccessibilityNode)>,
    mut lesson_buttons: Query<
        (
            Entity,
            &ClusterMember,
            &mut TabIndex,
            Has<InteractionDisabled>,
        ),
        With<WidgetButton>,
    >,
) {
    // Runs every frame on the tree, so every write below is guarded: once
    // no unit is mid-transition, an idle tree touches nothing (unguarded, it
    // re-ran transform and visibility propagation over every node).
    let step = transition_step(time.delta_secs(), reduced_motion.0);
    let moving = expansions
        .0
        .iter()
        .any(|(id, &amount)| expansion_after(amount, collapsed.0.contains(id), step) != amount);
    if moving {
        for (id, amount) in &mut expansions.0 {
            *amount = expansion_after(*amount, collapsed.0.contains(id), step);
        }
    }

    for (member, mut transform, mut visibility) in &mut members {
        let amount = expansions.0.get(&member.0).copied().unwrap_or(1.0);
        let eased = amount * amount * (3.0 - 2.0 * amount);
        let scale = Vec2::splat(eased.max(0.001));
        if transform.scale != scale {
            transform.scale = scale;
        }
        let wanted = if amount <= 0.0 {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }

    for (chevron, mut text) in &mut chevrons {
        let label = if collapsed.0.contains(&chevron.0) {
            "▶"
        } else {
            "▼"
        };
        if text.0 != label {
            text.0.clear();
            text.0.push_str(label);
        }
    }
    for (button, mut accessibility) in &mut unit_buttons {
        let expanded = !collapsed.0.contains(&button.0);
        if accessibility.is_expanded() != Some(expanded) {
            accessibility.set_expanded(expanded);
        }
    }
    for (entity, member, mut tab_index, disabled) in &mut lesson_buttons {
        let closing = collapsed.0.contains(&member.0);
        let wanted = if closing { -1 } else { 0 };
        if tab_index.0 != wanted {
            tab_index.0 = wanted;
        }
        match (closing, disabled) {
            (true, false) => {
                commands.entity(entity).insert(InteractionDisabled);
            }
            (false, true) => {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
            _ => {}
        }
    }
}

pub(super) fn slide_offset(slide: &UnitSlide) -> f32 {
    let eased = slide.amount * slide.amount * (3.0 - 2.0 * slide.amount);
    slide.from_px * (1.0 - eased)
}

pub(crate) fn animate_unit_slides(
    time: Res<Time>,
    reduced_motion: Res<ReducedMotion>,
    anchor: Res<PendingViewportAnchor>,
    mut slides: ResMut<UnitSlides>,
    mut owned: Query<(&LayoutOwner, &mut UiTransform), Without<MovingEdge>>,
    mut edges: Query<(&MovingEdge, &mut Node)>,
) {
    if slides.0.is_empty() {
        return;
    }
    // Hold every slide at its start until the anchor's scroll is applied:
    // `restore_viewport_anchor` re-bases them onto the new scroll, which is
    // only seamless while none has advanced. The offsets are still drawn,
    // so the rebuilt tree sits exactly where the old one was meanwhile.
    let step = if anchor.canvas_x.is_some() {
        0.0
    } else {
        transition_step(time.delta_secs(), reduced_motion.0)
    };
    for slide in slides.0.values_mut() {
        slide.amount = (slide.amount + step).min(1.0);
    }

    for (owner, mut transform) in &mut owned {
        let offset = slides.0.get(&owner.0).map_or(0.0, slide_offset);
        transform.translation = Val2::px(offset, 0.0);
    }
    for (edge, mut node) in &mut edges {
        let from_offset = slides.0.get(&edge.from_unit).map_or(0.0, slide_offset);
        let to_offset = slides.0.get(&edge.to_unit).map_or(0.0, slide_offset);
        let from = Endpoint {
            centre: edge.from.centre + Vec2::X * from_offset,
            ..edge.from
        };
        let to = Endpoint {
            centre: edge.to.centre + Vec2::X * to_offset,
            ..edge.to
        };
        set_edge_geometry(&mut node, from, to, edge.thickness);
    }
    slides.0.retain(|_, slide| slide.amount < 1.0);
}

pub(crate) fn compact_finished_units(
    expansions: Res<UnitExpansions>,
    mut pending: ResMut<PendingCompaction>,
    mut page: ResMut<NextState<MenuPage>>,
) {
    if pending.0.is_empty() {
        return;
    }
    let all_closed = pending
        .0
        .iter()
        .all(|id| expansions.0.get(id).is_none_or(|amount| *amount <= 0.0));
    if all_closed {
        pending.0.clear();
        page.set(MenuPage::LessonTree);
    }
}

pub(super) fn expansion_after(current: f32, collapsed: bool, step: f32) -> f32 {
    let target = if collapsed { 0.0 } else { 1.0 };
    if current < target {
        (current + step).min(target)
    } else if current > target {
        (current - step).max(target)
    } else {
        current
    }
}

#[cfg(test)]
mod motion_tests {
    use super::*;

    /// One frame of the two transition systems, with a unit collapsing and
    /// another sliding. `Time`'s default delta is zero, so only Reduced
    /// Motion can move anything.
    fn one_frame(reduced_motion: bool) -> App {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<PendingViewportAnchor>()
            .insert_resource(ReducedMotion(reduced_motion))
            .insert_resource(CollapsedUnits(HashSet::from(["closing".to_string()])))
            .insert_resource(UnitExpansions(HashMap::from([(
                "closing".to_string(),
                1.0,
            )])))
            .insert_resource(UnitSlides(HashMap::from([(
                "sliding".to_string(),
                UnitSlide {
                    from_px: 120.0,
                    amount: 0.0,
                },
            )])))
            .add_systems(Update, (animate_unit_expansion, animate_unit_slides));
        app.update();
        app
    }

    #[test]
    fn reduced_motion_jumps_every_transition_to_its_end_on_the_first_frame() {
        let app = one_frame(true);
        assert_eq!(app.world().resource::<UnitExpansions>().0["closing"], 0.0);
        assert!(
            app.world().resource::<UnitSlides>().0.is_empty(),
            "a finished slide is dropped"
        );
    }

    #[test]
    fn slides_wait_for_the_anchor_scroll_before_advancing() {
        // Reduced Motion would finish a slide in one frame, so it makes the
        // hold visible: with the anchor still pending, nothing advances.
        let mut app = App::new();
        app.init_resource::<Time>()
            .insert_resource(ReducedMotion(true))
            .insert_resource(PendingViewportAnchor {
                unit_id: Some("toggled".to_string()),
                canvas_x: Some(600.0),
                ..default()
            })
            .insert_resource(UnitSlides(HashMap::from([(
                "sliding".to_string(),
                UnitSlide {
                    from_px: 120.0,
                    amount: 0.0,
                },
            )])))
            .add_systems(Update, animate_unit_slides);
        app.update();
        assert_eq!(
            app.world().resource::<UnitSlides>().0["sliding"].amount,
            0.0
        );
    }

    /// Where a unit is drawn on screen: its canvas position plus its slide
    /// offset, minus the scroll.
    fn screen_x(canvas_x: f32, slide: Option<&UnitSlide>, scroll: f32) -> f32 {
        canvas_x + slide.map_or(0.0, slide_offset) - scroll
    }

    #[test]
    fn re_basing_slides_onto_the_new_scroll_moves_nothing_on_screen() {
        // Expanding `toggled` widens its cluster: it moves 100 px right in
        // canvas coordinates, `right` 200 px, and `left` not at all. The
        // anchor then scrolls 100 px so `toggled` keeps its screen spot.
        let old = [("left", 200.0), ("toggled", 500.0), ("right", 900.0)];
        let new = [("left", 200.0), ("toggled", 600.0), ("right", 1_100.0)];
        let (old_scroll, new_scroll) = (300.0, 400.0);

        let mut slides: HashMap<String, UnitSlide> = old
            .iter()
            .zip(&new)
            .filter(|((_, from), (_, to))| from != to)
            .map(|((id, from), (_, to))| {
                (
                    id.to_string(),
                    UnitSlide {
                        from_px: from - to,
                        amount: 0.0,
                    },
                )
            })
            .collect();
        let ids: Vec<String> = new.iter().map(|(id, _)| id.to_string()).collect();
        shift_slides(&mut slides, &ids, new_scroll - old_scroll);

        for ((id, before), (_, after)) in old.iter().zip(&new) {
            assert_eq!(
                screen_x(*after, slides.get(*id), new_scroll),
                screen_x(*before, None, old_scroll),
                "{id} must start its slide where it was drawn"
            );
        }
        assert!(
            !slides.contains_key("toggled"),
            "the anchored unit does not move at all"
        );
        assert!(
            slides.contains_key("left"),
            "an unmoved unit still needs a slide to absorb the scroll"
        );
    }

    #[test]
    fn without_reduced_motion_transitions_advance_with_time() {
        let app = one_frame(false);
        assert_eq!(app.world().resource::<UnitExpansions>().0["closing"], 1.0);
        assert_eq!(
            app.world().resource::<UnitSlides>().0["sliding"].amount,
            0.0
        );
    }
}

#[cfg(test)]
mod viewport_tests {
    use super::*;

    #[test]
    fn viewport_position_survives_after_the_scroller_is_gone() {
        let mut app = App::new();
        app.init_resource::<LessonTreeViewport>()
            .add_systems(Update, remember_viewport);
        let scroller = app
            .world_mut()
            .spawn((LessonTreeScroller, ScrollPosition(Vec2::new(420.0, 180.0))))
            .id();

        app.update();
        app.world_mut().despawn(scroller);
        app.update();

        assert_eq!(
            app.world().resource::<LessonTreeViewport>().0,
            Vec2::new(420.0, 180.0),
        );
    }

    #[test]
    fn lesson_focus_centres_and_clamps_to_the_scrollable_range() {
        let viewport = Vec2::new(300.0, 200.0);
        let content = Vec2::new(1_000.0, 600.0);

        assert_eq!(
            centred_scroll(Vec2::new(500.0, 300.0), viewport, content),
            Vec2::new(350.0, 200.0),
        );
        assert_eq!(
            centred_scroll(Vec2::new(20.0, 590.0), viewport, content),
            Vec2::new(0.0, 400.0),
        );
    }
}
