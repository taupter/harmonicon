// SPDX-License-Identifier: MIT

//! Short pitch history shared by signal-quality feedback and the bend-path UI.

use std::collections::VecDeque;

use super::*;

const TRACE_HISTORY_SECS: f32 = 3.0;
const STABILITY_HISTORY_SECS: f32 = 0.30;
const MIN_STABILITY_SPAN_SECS: f32 = 0.12;
const MIN_STABILITY_SAMPLES: usize = 5;
const UNSTABLE_RESIDUAL_CENTS: f32 = 10.0;

#[derive(Clone, Copy, Debug)]
pub(super) struct TraceSample {
    pub time: f32,
    pub target_cents: f32,
}

/// Recent selected-hole pitch motion. A line-fit residual separates a smooth
/// bend from frame-to-frame detector jitter: deliberate travel may be steep,
/// but it still follows a coherent path.
#[derive(Resource, Default)]
pub struct BendTrace {
    samples: VecDeque<TraceSample>,
    elapsed: f32,
    last_hole: Option<u8>,
    last_technique: Option<Technique>,
    last_key: String,
    pub unstable: bool,
}

const TRACE_DOTS: usize = 32;
const RAIL_START_PERCENT: f32 = 8.0;
const RAIL_END_PERCENT: f32 = 92.0;

#[derive(Component)]
pub struct BendTraceDot(pub usize);
#[derive(Component)]
pub struct BendLiveMarker;
#[derive(Component)]
pub struct BendTargetBand;
#[derive(Component)]
pub struct BendNaturalLabel;
#[derive(Component)]
pub struct BendTargetLabel;

pub(super) fn spawn_bend_rail(card: &mut ChildSpawnerCommands, loc: &Localization) {
    card.spawn(Node {
        width: Val::Percent(100.0),
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    })
    .with_children(|labels| {
        labels.spawn((
            Text::new(loc.msg("bending-rail-natural")),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(0.70, 0.74, 0.80)),
            BendNaturalLabel,
        ));
        labels.spawn((
            Text::new(loc.msg("bending-rail-target")),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(0.70, 0.84, 0.74)),
            BendTargetLabel,
        ));
    });
    card.spawn((
        Node {
            position_type: PositionType::Relative,
            width: Val::Percent(100.0),
            height: Val::Px(48.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.07, 0.10, 0.90)),
    ))
    .with_children(|rail| {
        rail.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(RAIL_START_PERCENT),
                top: Val::Px(23.0),
                width: Val::Percent(RAIL_END_PERCENT - RAIL_START_PERCENT),
                height: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.35, 0.38, 0.44)),
        ));
        rail.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(11.0),
                height: Val::Px(26.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.25, 0.75, 0.38, 0.20)),
            BendTargetBand,
        ));
        for index in 0..TRACE_DOTS {
            rail.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(21.0),
                    width: Val::Px(6.0),
                    height: Val::Px(6.0),
                    border_radius: BorderRadius::all(Val::Percent(50.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.45, 0.72, 0.95, 0.0)),
                Visibility::Hidden,
                BendTraceDot(index),
            ));
        }
        rail.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(18.0),
                width: Val::Px(12.0),
                height: Val::Px(12.0),
                border_radius: BorderRadius::all(Val::Percent(50.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.92, 0.94, 1.0)),
            Visibility::Hidden,
            BendLiveMarker,
        ));
    });
}

pub(super) fn rail_percent(target_cents: f32, natural_target_cents: f32) -> f32 {
    if natural_target_cents.abs() < 1.0 {
        return (RAIL_START_PERCENT + RAIL_END_PERCENT) * 0.5;
    }
    let progress = (natural_target_cents - target_cents) / natural_target_cents;
    (RAIL_START_PERCENT + progress * (RAIL_END_PERCENT - RAIL_START_PERCENT)).clamp(2.0, 98.0)
}

pub(super) fn residual_rms(samples: &VecDeque<TraceSample>) -> Option<f32> {
    if samples.len() < MIN_STABILITY_SAMPLES {
        return None;
    }
    let span = samples.back()?.time - samples.front()?.time;
    if span < MIN_STABILITY_SPAN_SECS {
        return None;
    }

    let n = samples.len() as f32;
    let mean_t = samples.iter().map(|sample| sample.time).sum::<f32>() / n;
    let mean_c = samples
        .iter()
        .map(|sample| sample.target_cents)
        .sum::<f32>()
        / n;
    let variance_t = samples
        .iter()
        .map(|sample| (sample.time - mean_t).powi(2))
        .sum::<f32>();
    if variance_t <= f32::EPSILON {
        return None;
    }
    let slope = samples
        .iter()
        .map(|sample| (sample.time - mean_t) * (sample.target_cents - mean_c))
        .sum::<f32>()
        / variance_t;
    let residual = samples
        .iter()
        .map(|sample| {
            let fitted = mean_c + slope * (sample.time - mean_t);
            (sample.target_cents - fitted).powi(2)
        })
        .sum::<f32>();
    Some((residual / n).sqrt())
}

pub fn update_bend_trace(
    key: Res<TrainerKey>,
    target: Res<TrainerTarget>,
    active: Res<ActivePitches>,
    time: Res<Time>,
    mut trace: ResMut<BendTrace>,
) {
    let target_changed = trace.last_hole != Some(target.hole)
        || trace.last_technique != Some(target.technique)
        || trace.last_key != key.0;
    if target_changed {
        trace.samples.clear();
        trace.unstable = false;
        trace.last_hole = Some(target.hole);
        trace.last_technique = Some(target.technique);
        trace.last_key.clone_from(&key.0);
    }

    trace.elapsed += time.delta_secs();
    let harp = richter_harp(&key.0);
    match tuner_observation(&harp, *target, &active) {
        Some(TunerObservation::TargetFamily(target_cents)) => {
            let elapsed = trace.elapsed;
            trace.samples.push_back(TraceSample {
                time: elapsed,
                target_cents,
            });
            while trace
                .samples
                .front()
                .is_some_and(|sample| elapsed - sample.time > TRACE_HISTORY_SECS)
            {
                trace.samples.pop_front();
            }
            let stability_samples = trace
                .samples
                .iter()
                .filter(|sample| elapsed - sample.time <= STABILITY_HISTORY_SECS)
                .copied()
                .collect();
            trace.unstable = residual_rms(&stability_samples)
                .is_some_and(|residual| residual > UNSTABLE_RESIDUAL_CENTS);
        }
        _ => {
            trace.samples.clear();
            trace.unstable = false;
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn update_bend_rail(
    key: Res<TrainerKey>,
    target: Res<TrainerTarget>,
    trace: Res<BendTrace>,
    loc: Res<Localization>,
    mut dots: Query<(
        &BendTraceDot,
        &mut Node,
        &mut BackgroundColor,
        &mut Visibility,
    )>,
    mut marker: Query<(&mut Node, &mut Visibility), (With<BendLiveMarker>, Without<BendTraceDot>)>,
    mut band: Query<&mut Node, (With<BendTargetBand>, Without<BendLiveMarker>)>,
    mut natural_labels: Query<&mut Text, (With<BendNaturalLabel>, Without<BendTargetLabel>)>,
    mut target_labels: Query<&mut Text, (With<BendTargetLabel>, Without<BendNaturalLabel>)>,
) {
    let harp = richter_harp(&key.0);
    let Some(target_note) = target_note(&harp, *target) else {
        return;
    };
    let Some(natural_note) = natural_note_for_target(&harp, *target) else {
        return;
    };
    let (Some(target_freq), Some(natural_freq)) =
        (note_freq_hz(&target_note), note_freq_hz(&natural_note))
    else {
        return;
    };
    let natural_cents = 1200.0 * (natural_freq / target_freq).log2();

    for mut text in &mut natural_labels {
        *text = Text::new(String::from(loc.msg_args(
            "bending-rail-natural-note",
            &[("note", natural_note.clone())],
        )));
    }
    for mut text in &mut target_labels {
        *text = Text::new(String::from(
            loc.msg_args("bending-rail-target-note", &[("note", target_note.clone())]),
        ));
    }
    let band_width = if natural_cents.abs() < 1.0 {
        10.0
    } else {
        (2.0 * IN_TUNE_CENTS / natural_cents.abs() * (RAIL_END_PERCENT - RAIL_START_PERCENT))
            .clamp(3.0, 20.0)
    };
    for mut node in &mut band {
        node.left = Val::Percent(rail_percent(0.0, natural_cents) - band_width * 0.5);
        node.width = Val::Percent(band_width);
    }

    let sample_step = trace.samples.len().div_ceil(TRACE_DOTS).max(1);
    let visible: Vec<_> = trace
        .samples
        .iter()
        .rev()
        .step_by(sample_step)
        .take(TRACE_DOTS)
        .collect();
    for (dot, mut node, mut color, mut visibility) in &mut dots {
        let Some(sample) = visible.get(dot.0) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        node.left = Val::Percent(rail_percent(sample.target_cents, natural_cents));
        let age = trace.elapsed - sample.time;
        let alpha = (1.0 - age / TRACE_HISTORY_SECS).clamp(0.08, 0.72);
        color.0 = Color::srgba(0.45, 0.72, 0.95, alpha);
        *visibility = Visibility::Visible;
    }
    let Ok((mut node, mut visibility)) = marker.single_mut() else {
        return;
    };
    if let Some(sample) = trace.samples.back() {
        node.left = Val::Percent(rail_percent(sample.target_cents, natural_cents));
        *visibility = Visibility::Visible;
    } else {
        *visibility = Visibility::Hidden;
    }
}
