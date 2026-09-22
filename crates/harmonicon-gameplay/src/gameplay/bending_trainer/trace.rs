// SPDX-License-Identifier: MIT

//! Short pitch history shared by signal-quality feedback and the bend-path UI.

use std::collections::VecDeque;

use super::*;

const HISTORY_SECS: f32 = 0.30;
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
                .is_some_and(|sample| elapsed - sample.time > HISTORY_SECS)
            {
                trace.samples.pop_front();
            }
            trace.unstable = residual_rms(&trace.samples)
                .is_some_and(|residual| residual > UNSTABLE_RESIDUAL_CENTS);
        }
        _ => {
            trace.samples.clear();
            trace.unstable = false;
        }
    }
}
