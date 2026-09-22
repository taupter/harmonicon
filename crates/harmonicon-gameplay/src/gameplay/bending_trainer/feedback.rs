// SPDX-License-Identifier: MIT

//! Selected-hole pitch classification and the optional natural-note check.

use super::*;

pub(super) const IN_TUNE_CENTS: f32 = 6.0;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum TunerObservation {
    Silent,
    WrongPitch(String),
    TargetFamily(f32),
}

pub(super) fn natural_note_for_target(harp: &Harmonica, target: TrainerTarget) -> Option<String> {
    let holes = hole_notes(harp, target.hole);
    match target.technique {
        Technique::Blow => holes.blow,
        Technique::Draw => holes.draw,
        Technique::Bend1 | Technique::Bend2 | Technique::Bend3 if target.hole <= 6 => holes.draw,
        Technique::Bend1 | Technique::Bend2 | Technique::Bend3 => holes.blow,
        Technique::Over if target.hole <= 6 => holes.blow,
        Technique::Over => holes.draw,
    }
}

pub fn update_natural_check(
    key: Res<TrainerKey>,
    target: Res<TrainerTarget>,
    active: Res<ActivePitches>,
    time: Res<Time>,
    mut check: ResMut<NaturalCheck>,
) {
    if target.is_changed() {
        *check = NaturalCheck::default();
        return;
    }
    if !check.requested || check.confirmed {
        return;
    }
    let harp = richter_harp(&key.0);
    let Some(note) = natural_note_for_target(&harp, *target) else {
        return;
    };
    let Some(midi) = note_to_midi(&note).map(|midi| midi as u8) else {
        return;
    };
    let Some(freq) = note_freq_hz(&note) else {
        return;
    };
    let centered = active.0.iter().any(|pitch| {
        pitch.midi == midi && (1200.0 * (pitch.frequency / freq).log2()).abs() <= 12.0
    });
    check.hold_secs = if centered {
        check.hold_secs + time.delta_secs()
    } else {
        0.0
    };
    check.confirmed = check.hold_secs >= 0.35;
}

pub fn update_natural_check_label(
    check: Res<NaturalCheck>,
    target: Res<TrainerTarget>,
    key: Res<TrainerKey>,
    loc: Res<Localization>,
    mut labels: Query<&mut Text, With<NaturalCheckLabel>>,
) {
    if !check.is_changed() && !target.is_changed() && !key.is_changed() {
        return;
    }
    let note =
        natural_note_for_target(&richter_harp(&key.0), *target).unwrap_or_else(|| "?".to_string());
    let key = if check.confirmed {
        "bending-check-natural-ready"
    } else if check.requested {
        "bending-check-natural-listening"
    } else {
        "bending-check-natural-idle"
    };
    for mut text in &mut labels {
        *text = Text::new(String::from(loc.msg_args(key, &[("note", note.clone())])));
    }
}

pub(super) fn tuner_observation(
    harp: &Harmonica,
    target: TrainerTarget,
    active: &ActivePitches,
) -> Option<TunerObservation> {
    let target_freq = note_freq_hz(&target_note(harp, target)?)?;
    if active.0.is_empty() {
        return Some(TunerObservation::Silent);
    }
    let holes = hole_notes(harp, target.hole);
    let family: HashSet<u8> = holes
        .blow
        .iter()
        .chain(holes.draw.iter())
        .chain(holes.bends.iter())
        .chain(holes.over.iter())
        .filter_map(|note| note_to_midi(note).map(|midi| midi as u8))
        .collect();
    let by_distance = |a: &&harmonicon_audio::pitch_detect::PitchInfo,
                       b: &&harmonicon_audio::pitch_detect::PitchInfo| {
        (a.frequency.log2() - target_freq.log2())
            .abs()
            .total_cmp(&(b.frequency.log2() - target_freq.log2()).abs())
    };
    if let Some(heard) = active
        .0
        .iter()
        .filter(|p| family.contains(&p.midi))
        .min_by(by_distance)
    {
        return Some(TunerObservation::TargetFamily(
            1200.0 * (heard.frequency / target_freq).log2(),
        ));
    }
    active
        .0
        .iter()
        .min_by(by_distance)
        .map(|heard| TunerObservation::WrongPitch(format!("{}{}", heard.note, heard.octave)))
}

pub fn update_tuner_readout(
    key: Res<TrainerKey>,
    target: Res<TrainerTarget>,
    active: Res<ActivePitches>,
    loc: Res<Localization>,
    mut labels: Query<(&mut Text, &mut TextColor), With<TunerReadout>>,
) {
    let Ok((mut text, mut color)) = labels.single_mut() else {
        return;
    };
    let harp = richter_harp(&key.0);
    let Some(target_note) = target_note(&harp, *target) else {
        *text = Text::new(String::from(loc.msg("bending-no-note-for-technique")));
        color.0 = Color::srgb(0.60, 0.60, 0.65);
        return;
    };
    let Some(observation) = tuner_observation(&harp, *target, &active) else {
        return;
    };
    let cents = match observation {
        TunerObservation::Silent => {
            *text = Text::new(String::from(
                loc.msg_args("bending-play-it-target", &[("note", target_note)]),
            ));
            color.0 = Color::srgb(0.60, 0.60, 0.65);
            return;
        }
        TunerObservation::WrongPitch(heard) => {
            *text = Text::new(String::from(loc.msg_args(
                "bending-wrong-pitch",
                &[("note", heard), ("hole", target.hole.to_string())],
            )));
            color.0 = Color::srgb(0.90, 0.60, 0.30);
            return;
        }
        TunerObservation::TargetFamily(cents) => cents,
    };
    let (key, color_value) = if cents.abs() <= IN_TUNE_CENTS {
        ("bending-in-tune", Color::srgb(0.45, 0.85, 0.50))
    } else if cents > 0.0 {
        ("bending-cents-sharp", Color::srgb(0.90, 0.70, 0.30))
    } else {
        ("bending-cents-flat", Color::srgb(0.90, 0.70, 0.30))
    };
    let args = &[("cents", format!("{cents:+.0}")), ("note", target_note)];
    *text = Text::new(String::from(loc.msg_args(key, args)));
    color.0 = color_value;
}
