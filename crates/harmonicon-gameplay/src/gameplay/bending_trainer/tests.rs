// SPDX-License-Identifier: MIT

use super::*;

// ── row_to_technique (diagram click → drill target) ──────────────────────

#[test]
fn plain_blow_and_draw_rows_map_directly() {
    assert_eq!(row_to_technique(Row::Blow), Some(Technique::Blow));
    assert_eq!(row_to_technique(Row::Draw), Some(Technique::Draw));
}

#[test]
fn both_bend_wings_collapse_to_the_same_depth() {
    // The diagram distinguishes which wing a bend is on (to read the
    // right reed); the drill target doesn't need that, just the depth.
    for (blow, draw, expected) in [
        (Row::BlowBend(0), Row::DrawBend(0), Technique::Bend1),
        (Row::BlowBend(1), Row::DrawBend(1), Technique::Bend2),
        (Row::BlowBend(2), Row::DrawBend(2), Technique::Bend3),
    ] {
        assert_eq!(row_to_technique(blow), Some(expected));
        assert_eq!(row_to_technique(draw), Some(expected));
    }
}

// ── Drill progress grid (drill_accuracy, progress_tint) ──────────────────

#[test]
fn accuracy_is_none_for_a_never_attempted_target() {
    assert_eq!(drill_accuracy(None), None);
    assert_eq!(
        drill_accuracy(Some(&DrillStat {
            attempts: 0,
            hits: 0
        })),
        None
    );
}

#[test]
fn accuracy_is_hit_rate_for_an_attempted_target() {
    assert_eq!(
        drill_accuracy(Some(&DrillStat {
            attempts: 4,
            hits: 3
        })),
        Some(0.75)
    );
    assert_eq!(
        drill_accuracy(Some(&DrillStat {
            attempts: 5,
            hits: 0
        })),
        Some(0.0)
    );
}

#[test]
fn untried_tint_matches_the_diagram_idle_color() {
    assert_eq!(progress_tint(None), CELL_DEFAULT);
}

#[test]
fn tint_reddens_toward_zero_and_greens_toward_one() {
    let weak = progress_tint(Some(0.0)).to_srgba();
    let strong = progress_tint(Some(1.0)).to_srgba();
    assert!(weak.red > weak.green, "0% accuracy should read as red");
    assert!(
        strong.green > strong.red,
        "100% accuracy should read as green"
    );
    // Never attempted stays visually distinct from a 0%-accuracy target.
    assert_ne!(progress_tint(Some(0.0)), progress_tint(None));
}

#[test]
fn overblow_and_overdraw_rows_both_map_to_over() {
    assert_eq!(row_to_technique(Row::Overblow), Some(Technique::Over));
    assert_eq!(row_to_technique(Row::Overdraw), Some(Technique::Over));
}

// ── Drill persistence (Technique storage key, stats <-> profile) ─────────

#[test]
fn every_technique_storage_key_round_trips() {
    for &t in &ALL_TECHNIQUES {
        assert_eq!(Technique::from_storage_key(t.storage_key()), Some(t));
    }
}

#[test]
fn unknown_storage_key_is_none() {
    assert_eq!(Technique::from_storage_key("nonsense"), None);
}

#[test]
fn drill_key_combines_hole_and_technique() {
    assert_eq!(drill_key(2, Technique::Bend1), "2:bend1");
    assert_eq!(drill_key(10, Technique::Over), "10:over");
}

#[test]
fn stats_round_trip_through_the_profile_shape() {
    let mut stats = std::collections::HashMap::new();
    stats.insert(
        (2u8, Technique::Bend1),
        DrillStat {
            attempts: 5,
            hits: 3,
        },
    );
    stats.insert(
        (9u8, Technique::Over),
        DrillStat {
            attempts: 1,
            hits: 0,
        },
    );

    let profile = stats_to_profile(&stats);
    assert_eq!(profile.len(), 2);
    assert_eq!(
        profile["2:bend1"],
        DrillRecord {
            attempts: 5,
            hits: 3
        }
    );

    let restored = stats_from_profile(&profile);
    assert_eq!(restored.len(), 2);
    assert_eq!(restored[&(2, Technique::Bend1)].attempts, 5);
    assert_eq!(restored[&(2, Technique::Bend1)].hits, 3);
    assert_eq!(restored[&(9, Technique::Over)].attempts, 1);
}

#[test]
fn unparsable_profile_entries_are_dropped_not_fatal() {
    let mut profile = std::collections::HashMap::new();
    profile.insert(
        "not-a-key".to_string(),
        DrillRecord {
            attempts: 1,
            hits: 1,
        },
    );
    profile.insert(
        "2:not-a-technique".to_string(),
        DrillRecord {
            attempts: 1,
            hits: 1,
        },
    );
    profile.insert(
        "2:bend1".to_string(),
        DrillRecord {
            attempts: 4,
            hits: 2,
        },
    );
    let stats = stats_from_profile(&profile);
    assert_eq!(
        stats.len(),
        1,
        "only the one well-formed entry should survive"
    );
    assert!(stats.contains_key(&(2, Technique::Bend1)));
}

// `key_offset` is now `harmonicon_core::harmonica::key_offset` — its
// octave-folding behaviour is tested there, not duplicated here.
// `richter_harp`'s own reference layout (C/D/G hole-1 pitches) is tested
// once, centrally, in `song::harmonica::tests` — not duplicated here; this
// module's own tests below only care that bending-trainer logic built atop
// `richter_harp` (target resolution, valid targets, hints) behaves
// correctly, not that `richter_harp` itself is correct.

#[test]
fn target_note_reads_the_right_technique_off_the_harp() {
    let harp = richter_harp("C");
    // Hole 1: blow C4, draw D4, single ½-step bend C#4, overblow D#4.
    assert_eq!(
        target_note(
            &harp,
            TrainerTarget {
                hole: 1,
                technique: Technique::Blow
            }
        )
        .as_deref(),
        Some("C4")
    );
    assert_eq!(
        target_note(
            &harp,
            TrainerTarget {
                hole: 1,
                technique: Technique::Bend1
            }
        )
        .as_deref(),
        Some("C#4")
    );
    // Hole 5 has no bend (blow E5, draw F5 are a semitone apart).
    assert_eq!(
        target_note(
            &harp,
            TrainerTarget {
                hole: 5,
                technique: Technique::Bend1
            }
        ),
        None
    );
}

#[test]
fn natural_anchor_uses_the_reed_direction_that_creates_the_technique() {
    let harp = richter_harp("C");
    assert_eq!(
        natural_note_for_target(
            &harp,
            TrainerTarget {
                hole: 2,
                technique: Technique::Bend1,
            },
        )
        .as_deref(),
        Some("G4")
    );
    assert_eq!(
        natural_note_for_target(
            &harp,
            TrainerTarget {
                hole: 8,
                technique: Technique::Bend1,
            },
        )
        .as_deref(),
        Some("E6")
    );
    assert_eq!(
        natural_note_for_target(
            &harp,
            TrainerTarget {
                hole: 4,
                technique: Technique::Over,
            },
        )
        .as_deref(),
        Some("C5")
    );
    assert_eq!(
        natural_note_for_target(
            &harp,
            TrainerTarget {
                hole: 8,
                technique: Technique::Over,
            },
        )
        .as_deref(),
        Some("D6")
    );
}

#[test]
fn note_freq_hz_matches_concert_pitch() {
    assert!((note_freq_hz("A4").unwrap() - 440.0).abs() < 0.01);
    // One semitone below A4.
    assert!((note_freq_hz("G#4").unwrap() - 415.30).abs() < 0.1);
}

fn detected(note: &str) -> harmonicon_audio::pitch_detect::PitchInfo {
    let midi = note_to_midi(note).unwrap() as u8;
    let octave = midi as i32 / 12 - 1;
    let name = note
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .to_string();
    harmonicon_audio::pitch_detect::PitchInfo {
        midi,
        note: name,
        octave,
        frequency: harmonicon_core::midi::midi_to_freq_hz(midi as f32),
    }
}

#[test]
fn tuner_observation_distinguishes_silence_wrong_pitch_and_target_family() {
    let harp = richter_harp("C");
    let target = TrainerTarget {
        hole: 2,
        technique: Technique::Bend1,
    };
    assert_eq!(
        tuner_observation(&harp, target, &ActivePitches::default()),
        Some(TunerObservation::Silent)
    );
    assert_eq!(
        tuner_observation(&harp, target, &ActivePitches(vec![detected("C6")])),
        Some(TunerObservation::WrongPitch("C6".to_string()))
    );
    assert!(matches!(
        tuner_observation(&harp, target, &ActivePitches(vec![detected("F#4")])),
        Some(TunerObservation::TargetFamily(cents)) if cents.abs() < 0.1
    ));
}

#[test]
fn tuner_observation_accepts_the_natural_anchor_on_the_selected_hole() {
    let harp = richter_harp("C");
    let target = TrainerTarget {
        hole: 2,
        technique: Technique::Bend1,
    };
    assert!(matches!(
        tuner_observation(&harp, target, &ActivePitches(vec![detected("G4")])),
        Some(TunerObservation::TargetFamily(cents)) if cents > 90.0
    ));
}

#[test]
fn stability_fit_accepts_smooth_pitch_motion() {
    let samples = (0..=6)
        .map(|index| TraceSample {
            time: index as f32 * 0.025,
            target_cents: 100.0 - index as f32 * 12.0,
        })
        .collect();
    assert!(residual_rms(&samples).unwrap() < 0.01);
}

#[test]
fn stability_fit_rejects_jitter_around_a_pitch() {
    let cents = [0.0, 18.0, -17.0, 16.0, -19.0, 14.0, -15.0];
    let samples = cents
        .iter()
        .enumerate()
        .map(|(index, cents)| TraceSample {
            time: index as f32 * 0.025,
            target_cents: *cents,
        })
        .collect();
    assert!(residual_rms(&samples).unwrap() > 10.0);
}

#[test]
fn bend_rail_maps_natural_target_and_overshoot() {
    let natural_cents = 100.0;
    assert!((rail_percent(natural_cents, natural_cents) - 8.0).abs() < 0.01);
    assert!((rail_percent(0.0, natural_cents) - 92.0).abs() < 0.01);
    assert!(rail_percent(-25.0, natural_cents) > 92.0);
}

// ── technique_hint ────────────────────────────────────────────────────────

#[test]
fn blow_and_draw_hints_dont_depend_on_hole() {
    assert_eq!(
        technique_hint_key(Technique::Blow, 1),
        technique_hint_key(Technique::Blow, 9)
    );
    assert_eq!(
        technique_hint_key(Technique::Draw, 1),
        technique_hint_key(Technique::Draw, 9)
    );
}

#[test]
fn bend_hint_direction_matches_the_hole_side() {
    // Holes 1-6 bend by drawing; holes 7-10 bend by blowing.
    assert_eq!(
        technique_hint_key(Technique::Bend1, 3),
        "bending-technique-hint-draw-bend-half"
    );
    assert_eq!(
        technique_hint_key(Technique::Bend1, 8),
        "bending-technique-hint-blow-bend-half"
    );
}

#[test]
fn bend_hint_wording_deepens_with_technique() {
    assert_ne!(
        technique_hint_key(Technique::Bend1, 3),
        technique_hint_key(Technique::Bend2, 3)
    );
    assert_ne!(
        technique_hint_key(Technique::Bend2, 3),
        technique_hint_key(Technique::Bend3, 3)
    );
}

#[test]
fn over_hint_names_overblow_or_overdraw_by_hole() {
    // Overblow holes.
    for hole in [1, 4, 5, 6] {
        assert_eq!(
            technique_hint_key(Technique::Over, hole),
            "bending-technique-hint-overblow"
        );
    }
    // Overdraw holes.
    for hole in 7..=10 {
        assert_eq!(
            technique_hint_key(Technique::Over, hole),
            "bending-technique-hint-overdraw"
        );
    }
    // Holes 2 and 3 support neither.
    for hole in [2, 3] {
        assert_eq!(
            technique_hint_key(Technique::Over, hole),
            "bending-technique-hint-over-unsupported"
        );
    }
}

#[test]
fn drill_stat_weight_favors_never_seen_and_weak_targets() {
    let never = DrillStat::default();
    let mostly_missed = DrillStat {
        attempts: 10,
        hits: 1,
    };
    let mostly_hit = DrillStat {
        attempts: 10,
        hits: 9,
    };
    let perfect = DrillStat {
        attempts: 10,
        hits: 10,
    };
    assert!(mostly_missed.weight() > never.weight());
    assert!(never.weight() > mostly_hit.weight());
    assert!(mostly_hit.weight() > perfect.weight());
}

#[test]
fn valid_targets_excludes_bends_the_harp_cant_produce() {
    let harp = richter_harp("C");
    let targets = valid_targets(&harp);
    assert!(
        targets
            .iter()
            .any(|t| t.hole == 1 && t.technique == Technique::Blow)
    );
    // Hole 5 has no bend on a Richter-tuned harp.
    assert!(
        !targets
            .iter()
            .any(|t| t.hole == 5 && t.technique == Technique::Bend1)
    );
}

#[test]
fn pick_next_target_avoids_immediate_repeat_when_alternatives_exist() {
    let harp = richter_harp("C");
    let stats = std::collections::HashMap::new();
    let avoid = TrainerTarget {
        hole: 1,
        technique: Technique::Blow,
    };
    for _ in 0..20 {
        let picked = pick_next_target(&harp, &stats, Some(avoid)).expect("a target exists");
        assert_ne!(
            (picked.hole, picked.technique),
            (avoid.hole, avoid.technique)
        );
    }
}

// ── key_labels (Key combobox options) ─────────────────────────────────────

#[test]
fn key_labels_matches_the_12_chromatic_note_names() {
    let labels = key_labels();
    assert_eq!(labels.len(), 12);
    assert_eq!(
        labels,
        NOTE_NAMES.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    );
}
