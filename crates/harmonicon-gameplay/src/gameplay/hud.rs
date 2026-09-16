// SPDX-License-Identifier: MIT

//! The score HUD: marker components and the message-driven display update.
//! `judge::score_notes` emits a [`super::state::NoteScored`] message the
//! instant `Score` moves; [`update_score_display`] is a `MessageReader`
//! consumer, not a per-frame `format!` into `Text`.

use bevy::prelude::*;
use bevy_fluent::Localization;

use harmonicon_core::scoring::{HitQuality, combo_label, compute_multiplier};
use harmonicon_platform::localization::LocalizationExt;

use super::state::{
    FEEDBACK_FADE_SECS, HitFeedback, JudgmentFeedback, MissReason, NoteScored, Score, ScoringConfig,
};

// Score HUD marker components
#[derive(Component)]
pub struct ScoreText;
#[derive(Component)]
pub struct ComboText;
#[derive(Component)]
pub struct FeedbackText;

/// Localization key and tint for one judgment, shared by the label-once and
/// the per-frame color-fade halves of [`update_score_display`]. Every arm is a
/// straight lookup of a decision `score_notes` already made — the timing sign
/// that splits `Early` from `Late` is the offset the scorer classified from,
/// not a threshold reapplied here.
fn feedback_style(judgment: JudgmentFeedback) -> (&'static str, f32, f32, f32) {
    match judgment {
        JudgmentFeedback::Hit {
            quality: HitQuality::Perfect,
            ..
        } => ("gameplay-judgment-perfect", 1.00, 0.85, 0.10),
        JudgmentFeedback::Hit {
            quality: HitQuality::Good,
            offset,
        } if offset < 0.0 => ("gameplay-judgment-early", 0.40, 0.82, 1.00),
        JudgmentFeedback::Hit {
            quality: HitQuality::Good,
            offset,
        } if offset > 0.0 => ("gameplay-judgment-late", 1.00, 0.68, 0.28),
        JudgmentFeedback::Hit { .. } => ("gameplay-judgment-good", 0.40, 1.00, 0.35),
        JudgmentFeedback::Miss(MissReason::NoAttack) => {
            ("gameplay-judgment-no-attack", 1.00, 0.35, 0.35)
        }
        JudgmentFeedback::Miss(MissReason::WrongPitch) => {
            ("gameplay-judgment-wrong-pitch", 1.00, 0.35, 0.35)
        }
        JudgmentFeedback::Miss(MissReason::IncompleteChord) => {
            ("gameplay-judgment-incomplete-chord", 1.00, 0.42, 0.30)
        }
        JudgmentFeedback::TechniqueMiss => ("gameplay-judgment-technique", 1.00, 0.55, 0.25),
    }
}

/// The score/combo digits only get re-`format!`ed when [`NoteScored`] says
/// `Score` actually moved. The feedback label is set once, on the frame a
/// message carries a `judgment` — not every frame of its fade, which stays a
/// per-frame color/alpha animation driven off `HitFeedback`.
pub(crate) fn update_score_display(
    mut scored: MessageReader<NoteScored>,
    score: Res<Score>,
    config: Res<ScoringConfig>,
    loc: Res<Localization>,
    mut feedback: ResMut<HitFeedback>,
    time: Res<Time>,
    mut q_score: Query<&mut Text, (With<ScoreText>, Without<ComboText>, Without<FeedbackText>)>,
    mut q_combo: Query<&mut Text, (With<ComboText>, Without<ScoreText>, Without<FeedbackText>)>,
    mut q_feedback: Query<
        (&mut Text, &mut TextColor),
        (With<FeedbackText>, Without<ScoreText>, Without<ComboText>),
    >,
) {
    let mut score_moved = false;
    let mut fresh_judgment = None;
    for ev in scored.read() {
        score_moved = true;
        if ev.judgment.is_some() {
            fresh_judgment = ev.judgment;
        }
    }

    if score_moved {
        let points = format!("{}", score.points);
        for mut t in &mut q_score {
            if t.0 != points {
                t.0 = points.clone();
            }
        }

        // Same multiplier `score_notes` actually applies to points, so the HUD
        // can never show a number the score disagrees with.
        let multiplier = if config.combo_enabled {
            compute_multiplier(
                score.combo,
                config.base_multiplier,
                config.step_multiplier,
                config.max_multiplier,
            )
        } else {
            1.0
        };
        let combo = combo_label(score.combo, multiplier);
        for mut t in &mut q_combo {
            if t.0 != combo {
                t.0 = combo.clone();
            }
        }
    }

    if let Some(judgment) = fresh_judgment {
        let (label_key, ..) = feedback_style(judgment);
        let label = String::from(loc.msg(label_key));
        for (mut t, _) in &mut q_feedback {
            t.0 = label.clone();
        }
    }

    feedback.timer = (feedback.timer - time.delta_secs()).max(0.0);

    for (_, mut color) in &mut q_feedback {
        match feedback.judgment {
            None => {
                *color = TextColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
            }
            Some(judgment) => {
                let alpha = (feedback.timer / FEEDBACK_FADE_SECS).clamp(0.0, 1.0);
                // Scale up then fade: pulse from 1.4× down to 1× size isn't
                // easily done here, so we just fade alpha.
                let (_, r, g, b) = feedback_style(judgment);
                *color = TextColor(Color::srgba(r, g, b, alpha));
                if feedback.timer == 0.0 {
                    feedback.judgment = None;
                }
            }
        }
    }
}
