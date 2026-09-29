// SPDX-License-Identifier: MIT

//! The 2D highway's technique gauge: a small panel just above the hit line,
//! in the lane beside the note it coaches, shown from shortly before a
//! bend/vibrato/wah note arrives until its hold ends.
//!
//! - **Bend**: a track from the unbent note (top) to the target (bottom)
//!   with the on-target band marked; the marker is the player's pitch, so
//!   bending moves it down toward the band.
//! - **Vibrato / wah**: a reference pulse swinging at the chart's rate — the
//!   wobble to copy — and, once the note is held, the player's own pitch
//!   (vibrato) or loudness (wah) swinging beside it. The dimmed band is the
//!   swing too small for the judge to count.
//!
//! Every reading comes from `technique_cue`, over the same samples and
//! tolerances the judge scores with.

use bevy::prelude::*;

use harmonicon_audio::AudioSettings;
use harmonicon_platform::localization::{Localization, LocalizationExt};

use super::gameplay_2d::{HIT_H_PCT, NoteCueBadge, NoteHighway};
use super::judge::judged_instant;
use super::notes::SongNotes;
use super::state::{ActivePitches, HarmonicaPitchFilter, PlayedHarp};
use super::technique_cue::{
    BendAdvice, CoachMode, RateAdvice, bend_reading, bend_target_band, bend_track_pct,
    bend_track_range, beside_lane, coach_mode, coach_note, format_rate, measured_rate,
    min_swing_band, pitch_class_name, rate_advice, reference_swing, swing_track_pct, vibrato_swing,
    wah_swing,
};

/// The gauge panel's root, holding its parts' entities so the per-frame
/// update reaches them without a marker query each.
#[derive(Component)]
pub(super) struct TechniqueCoach {
    top_label: Entity,
    band: Entity,
    reference: Entity,
    marker: Entity,
    bottom_label: Entity,
    advice: Entity,
}

/// See-through enough that a note falling down the gauge's lane still reads.
const PANEL_BG: Color = Color::srgba(0.04, 0.05, 0.1, 0.6);
const TRACK_BG: Color = Color::srgba(1.0, 1.0, 1.0, 0.18);
const TARGET_BAND: Color = Color::srgba(0.35, 0.9, 0.45, 0.28);
const DEAD_BAND: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);
const REFERENCE: Color = Color::srgba(0.8, 0.85, 1.0, 0.7);
const MARKER_ON: Color = Color::srgb(0.6, 1.0, 0.65);
const MARKER_OFF: Color = Color::srgb(1.0, 0.72, 0.2);
const LABEL: Color = Color::srgba(0.92, 0.94, 1.0, 0.95);

/// Height of the band/pulse/marker bars, in px; they are centred on their
/// percentage by a negative top margin of half this.
const BAR_PX: f32 = 5.0;

/// Spawns the (hidden) gauge once per highway, as its child, so it is
/// cleaned up with it.
pub(super) fn spawn_technique_coach(
    mut commands: Commands,
    highways: Query<Entity, Added<NoteHighway>>,
) {
    for highway in &highways {
        let text = |commands: &mut Commands, size: f32| {
            commands
                .spawn((
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(size),
                        ..default()
                    },
                    TextColor(LABEL),
                    TextLayout::justify(Justify::Center),
                ))
                .id()
        };
        let bar = |commands: &mut Commands, color: Color, left: f32, width: f32| {
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(left),
                        width: Val::Px(width),
                        height: Val::Px(BAR_PX),
                        margin: UiRect::top(Val::Px(-BAR_PX * 0.5)),
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(color),
                ))
                .id()
        };
        let top_label = text(&mut commands, 13.0);
        let bottom_label = text(&mut commands, 13.0);
        let advice = text(&mut commands, 11.0);
        let band = bar(&mut commands, TARGET_BAND, -9.0, 24.0);
        let reference = bar(&mut commands, REFERENCE, -24.0, 12.0);
        let marker = bar(&mut commands, MARKER_OFF, -11.0, 28.0);
        // Outlined so it stays distinct inside the band it is aiming for.
        commands
            .entity(marker)
            .insert(Outline::new(Val::Px(1.0), Val::ZERO, Color::BLACK));
        let track = commands
            .spawn((
                Node {
                    width: Val::Px(6.0),
                    flex_grow: 1.0,
                    margin: UiRect::vertical(Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(TRACK_BG),
            ))
            .add_children(&[band, reference, marker])
            .id();
        let panel = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Percent(HIT_H_PCT),
                    height: Val::Percent(30.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(4.0)),
                    display: Display::None,
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                // Above the notes falling through the same lane.
                ZIndex(5),
                TechniqueCoach {
                    top_label,
                    band,
                    reference,
                    marker,
                    bottom_label,
                    advice,
                },
            ))
            .add_children(&[top_label, track, bottom_label, advice])
            .id();
        commands.entity(highway).add_child(panel);
    }
}

/// One bar's wanted state: `None` hides it, `Some((top_pct, height_px))`
/// shows it there.
type BarState = Option<(f32, Option<f32>)>;

fn apply_bar(node: &mut Node, state: BarState) {
    match state {
        None => {
            if node.display != Display::None {
                node.display = Display::None;
            }
        }
        Some((top_pct, height_pct)) => {
            node.display = Display::Flex;
            node.top = Val::Percent(top_pct);
            (node.height, node.margin) = match height_pct {
                Some(h) => (Val::Percent(h), UiRect::ZERO),
                None => (Val::Px(BAR_PX), UiRect::top(Val::Px(-BAR_PX * 0.5))),
            };
        }
    }
}

fn set_text(texts: &mut Query<&mut Text>, entity: Entity, wanted: &str) {
    if let Ok(mut text) = texts.get_mut(entity)
        && text.0 != wanted
    {
        text.0 = wanted.to_string();
    }
}

fn bend_advice_key(advice: BendAdvice) -> Option<&'static str> {
    match advice {
        BendAdvice::Silent => None,
        BendAdvice::BendMore => Some("coach-bend-more"),
        BendAdvice::OnTarget => Some("coach-on-target"),
        BendAdvice::TooFar => Some("coach-too-far"),
    }
}

fn rate_advice_key(advice: RateAdvice) -> &'static str {
    match advice {
        RateAdvice::FollowPulse => "coach-follow-pulse",
        RateAdvice::SwingMore => "coach-swing-more",
        RateAdvice::Faster => "coach-faster",
        RateAdvice::Slower => "coach-slower",
        RateAdvice::OnRate => "coach-on-rate",
    }
}

/// Moves the gauge to the note it should coach this frame and redraws it,
/// or hides it when no technique note is near.
pub(super) fn update_technique_coach(
    song_notes: Res<SongNotes>,
    clock: Res<super::GameplayClock>,
    audio: Res<AudioSettings>,
    pitch_filter: Res<HarmonicaPitchFilter>,
    active: Res<ActivePitches>,
    played: Res<PlayedHarp>,
    loc: Res<Localization>,
    lesson: Option<Res<harmonicon_song::lessons::LessonContext>>,
    mut coaches: Query<(&TechniqueCoach, &mut Node)>,
    mut parts: Query<(&mut Node, &mut BackgroundColor), Without<TechniqueCoach>>,
    mut texts: Query<&mut Text>,
    mut cues: Query<(&NoteCueBadge, &mut Visibility)>,
) {
    let Ok((coach, mut panel)) = coaches.single_mut() else {
        return;
    };
    let judged = judged_instant(clock.get(), &audio, Some(&pitch_filter));
    let target = match (&played.0, lesson.is_some_and(|l| l.aural)) {
        // An aural lesson hides the notes on purpose; the gauge would give
        // them away.
        (Some(harp), false) => {
            coach_note(&song_notes.notes, song_notes.cursor, judged).map(|i| (harp.hole_count(), i))
        }
        _ => None,
    };
    // The coached note's own cue sits in the gauge's lane and says what the
    // gauge now shows, so it steps aside while the gauge is up.
    for (cue, mut visibility) in &mut cues {
        let wanted = if target.is_some_and(|(_, i)| i == cue.note_id) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
    let Some((hole_count, index)) = target else {
        if panel.display != Display::None {
            panel.display = Display::None;
        }
        return;
    };
    let note = &song_notes.notes[index];
    let Some(mode) = coach_mode(note) else {
        return;
    };

    let lane_pct = 100.0 / f32::from(hole_count.max(1));
    panel.display = Display::Flex;
    panel.left = Val::Percent(f32::from(beside_lane(note.hole, hole_count) - 1) * lane_pct);
    panel.width = Val::Percent(lane_pct);

    let (top, bottom, band, reference, marker, marker_on, advice): (
        String,
        String,
        BarState,
        BarState,
        BarState,
        bool,
        Option<String>,
    ) = match mode {
        CoachMode::Bend { natural, target } => {
            let reading = bend_reading(active.0.iter().map(|p| p.frequency), natural, target);
            let (lo, hi) = bend_target_band(natural, target);
            let range = bend_track_range(natural, target);
            let band_top = bend_track_pct(lo, range);
            (
                pitch_class_name(natural),
                pitch_class_name(target),
                Some((band_top, Some(bend_track_pct(hi, range) - band_top))),
                None,
                reading.position.map(|p| (bend_track_pct(p, range), None)),
                reading.advice == BendAdvice::OnTarget,
                bend_advice_key(reading.advice).map(|k| String::from(loc.msg(k))),
            )
        }
        CoachMode::Vibrato { hz } | CoachMode::Wah { hz } => {
            let half = min_swing_band(mode);
            let band_top = swing_track_pct(half);
            let pulse = reference_swing(hz, judged - note.time);
            let (swing, advice) = if note.hit {
                let swing = match mode {
                    CoachMode::Wah { .. } => wah_swing(&note.amp_samples),
                    _ => vibrato_swing(&note.pitch_samples),
                };
                let (measured, samples) = measured_rate(mode, note);
                (swing, rate_advice(measured, hz, samples))
            } else {
                (None, RateAdvice::FollowPulse)
            };
            let name = match mode {
                CoachMode::Wah { .. } => "mod-wah",
                _ => "mod-vibrato",
            };
            (
                String::from(loc.msg_args("coach-rate", &[("rate", format_rate(hz))])),
                String::from(loc.msg(name)),
                Some((band_top, Some(swing_track_pct(-half) - band_top))),
                Some((swing_track_pct(pulse), None)),
                swing.map(|s| (swing_track_pct(s), None)),
                advice == RateAdvice::OnRate,
                Some(String::from(loc.msg(rate_advice_key(advice)))),
            )
        }
    };

    set_text(&mut texts, coach.top_label, &top);
    set_text(&mut texts, coach.bottom_label, &bottom);
    set_text(&mut texts, coach.advice, advice.as_deref().unwrap_or(""));

    let band_color = match mode {
        CoachMode::Bend { .. } => TARGET_BAND,
        _ => DEAD_BAND,
    };
    let marker_color = if marker_on { MARKER_ON } else { MARKER_OFF };
    for (entity, state, color) in [
        (coach.band, band, Some(band_color)),
        (coach.reference, reference, None),
        (coach.marker, marker, Some(marker_color)),
    ] {
        if let Ok((mut node, mut background)) = parts.get_mut(entity) {
            apply_bar(&mut node, state);
            if let Some(color) = color
                && background.0 != color
            {
                background.0 = color;
            }
        }
    }
}
