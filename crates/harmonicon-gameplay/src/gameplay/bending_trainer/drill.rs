// SPDX-License-Identifier: MIT

//! The adaptive drill: per-target hit/miss stats persisted on the profile,
//! the weighted pick of the next target, the hold-to-advance / timeout loop,
//! and the drill's own labels and button state.

use super::*;

// ── Adaptive drill mode ─────────────────────────────────────────────────────────

/// Per hole/technique hit-rate, used to weight which targets the drill
/// serves up next — misses come back around more often than notes the
/// player already nails.
#[derive(Default, Clone, Copy)]
pub struct DrillStat {
    pub attempts: u32,
    pub hits: u32,
}

impl DrillStat {
    /// Selection weight: never-seen and weak targets are drawn more often;
    /// a target the player consistently hits fades toward the 1.0 floor.
    pub(super) fn weight(&self) -> f32 {
        if self.attempts == 0 {
            return 2.5;
        }
        let miss_rate = 1.0 - self.hits as f32 / self.attempts as f32;
        1.0 + 3.0 * miss_rate
    }
}

/// Auto-advancing ear-training drill: picks a random hole/technique, waits
/// for the player to sustain it in tune, then moves on. Tracks a running
/// hit rate per target so weak spots come up more often than mastered ones.
#[derive(Resource, Default)]
pub struct DrillState {
    pub enabled: bool,
    pub stats: std::collections::HashMap<(u8, Technique), DrillStat>,
    /// How long the current target has been held in tune, in seconds.
    pub hold_secs: f32,
    /// How long the current target has been active at all, in seconds —
    /// resets the drill to a fresh target if the player gets stuck.
    pub elapsed_secs: f32,
    pub streak: u32,
}

/// A `"{hole}:{technique}"` key for [`PlayerProfile::drills`] — stats aren't
/// keyed by [`TrainerKey`], since the physical skill a (hole, technique) pair
/// drills is the same regardless of which key harp it's practiced on.
pub(super) fn drill_key(hole: u8, technique: Technique) -> String {
    format!("{hole}:{}", technique.storage_key())
}

/// Snapshot of in-memory drill stats into the flat, string-keyed shape
/// `PlayerProfile::drills` persists.
pub(super) fn stats_to_profile(
    stats: &std::collections::HashMap<(u8, Technique), DrillStat>,
) -> std::collections::HashMap<String, DrillRecord> {
    stats
        .iter()
        .map(|(&(hole, technique), stat)| {
            (
                drill_key(hole, technique),
                DrillRecord {
                    attempts: stat.attempts,
                    hits: stat.hits,
                },
            )
        })
        .collect()
}

/// Inverse of [`stats_to_profile`], for loading. Entries with a key that
/// doesn't parse (a hand-edited or future-version `profile.json`) are
/// silently dropped rather than failing the whole load.
pub(super) fn stats_from_profile(
    drills: &std::collections::HashMap<String, DrillRecord>,
) -> std::collections::HashMap<(u8, Technique), DrillStat> {
    drills
        .iter()
        .filter_map(|(key, record)| {
            let (hole_str, tech_str) = key.split_once(':')?;
            let hole: u8 = hole_str.parse().ok()?;
            let technique = Technique::from_storage_key(tech_str)?;
            Some((
                (hole, technique),
                DrillStat {
                    attempts: record.attempts,
                    hits: record.hits,
                },
            ))
        })
        .collect()
}

/// A (hole, technique)'s hit-rate, or `None` if it's never been attempted —
/// kept distinct from a `0.0` accuracy so [`progress_tint`] can tell "never
/// tried" (neutral) apart from "tried and always missed" (red).
pub(super) fn drill_accuracy(stat: Option<&DrillStat>) -> Option<f32> {
    let stat = stat?;
    if stat.attempts == 0 {
        return None;
    }
    Some(stat.hits as f32 / stat.attempts as f32)
}

/// Idle-cell background color for a (hole, technique)'s drill progress: the
/// diagram's ordinary idle color for "never attempted", blending from a dim
/// red (weak) to a dim green (mastered) as hit-rate climbs — so the same
/// diagram used to pick a drill target also doubles as a progress map, no
/// separate screen needed.
pub(super) fn progress_tint(accuracy: Option<f32>) -> Color {
    let Some(acc) = accuracy else {
        return CELL_DEFAULT;
    };
    let acc = acc.clamp(0.0, 1.0);
    let weak = Color::srgb(0.32, 0.12, 0.12).to_srgba();
    let strong = Color::srgb(0.14, 0.34, 0.16).to_srgba();
    Color::srgb(
        weak.red + (strong.red - weak.red) * acc,
        weak.green + (strong.green - weak.green) * acc,
        weak.blue + (strong.blue - weak.blue) * acc,
    )
}

/// Tints every idle diagram cell by its drill accuracy (see [`progress_tint`]),
/// layered on top of [`harmonica_overlay::update_harmonica_overlay`]'s live
/// mic highlight — both write `BackgroundColor`, so this must run `.after`
/// it (enforced in `GameplayPlugin::build`) and skips any cell currently lit
/// by a sounding pitch, letting the live highlight win.
pub fn update_drill_progress_tint(
    active: Res<ActivePitches>,
    drill: Res<DrillState>,
    mut cells: Query<(&HarpOverlayCell, &DiagramCellTarget, &mut BackgroundColor)>,
) {
    let played: HashSet<u8> = active.0.iter().map(|p| p.midi).collect();
    for (cell, target, mut bg) in &mut cells {
        if cell.midi.is_some_and(|m| played.contains(&m)) {
            continue;
        }
        let Some(technique) = row_to_technique(target.row) else {
            continue;
        };
        let accuracy = drill_accuracy(drill.stats.get(&(target.hole, technique)));
        *bg = BackgroundColor(progress_tint(accuracy));
    }
}

/// Seconds the player must hold a target in tune before the drill advances.
pub(super) const DRILL_HOLD_TO_ADVANCE: f32 = 0.5;
/// Seconds before a stuck target is scored as a miss and swapped out.
pub(super) const DRILL_TIMEOUT_SECS: f32 = 12.0;

/// Every (hole, technique) pair the current harp can actually produce.
pub(super) fn valid_targets(harp: &Harmonica) -> Vec<TrainerTarget> {
    (1..=10)
        .flat_map(|hole| {
            ALL_TECHNIQUES
                .iter()
                .map(move |&technique| TrainerTarget { hole, technique })
        })
        .filter(|t| target_note(harp, *t).is_some())
        .collect()
}

/// Weighted-random pick of the next drill target, biased toward targets the
/// player has missed more often, and avoiding an immediate repeat of `avoid`
/// when another option exists.
pub(super) fn pick_next_target(
    harp: &Harmonica,
    stats: &std::collections::HashMap<(u8, Technique), DrillStat>,
    avoid: Option<TrainerTarget>,
) -> Option<TrainerTarget> {
    let mut pool = valid_targets(harp);
    if pool.len() > 1 {
        pool.retain(|&t| Some((t.hole, t.technique)) != avoid.map(|a| (a.hole, a.technique)));
    }
    if pool.is_empty() {
        return None;
    }
    let weights: Vec<f32> = pool
        .iter()
        .map(|t| {
            stats
                .get(&(t.hole, t.technique))
                .copied()
                .unwrap_or_default()
                .weight()
        })
        .collect();
    let total: f32 = weights.iter().sum();
    let mut roll = rand::random_range(0.0..total);
    for (target, w) in pool.iter().zip(weights.iter()) {
        if roll < *w {
            return Some(*target);
        }
        roll -= w;
    }
    pool.last().copied()
}

/// Note name for the current target on `harp`, or `None` if that hole doesn't
/// have that technique (e.g. hole 5 has no bend, most holes have no overblow).
pub(super) fn target_note(harp: &Harmonica, target: TrainerTarget) -> Option<String> {
    let holes = hole_notes(harp, target.hole);
    target.technique.note(&holes).map(str::to_string)
}

/// Frequency in Hz for a note label like `"C#4"`.
pub(super) fn note_freq_hz(note: &str) -> Option<f32> {
    harmonicon_core::midi::note_to_freq_hz(note)
}

/// A short, clean reference tone (fundamental + two soft harmonics) — plain
/// enough to make the *pitch* the whole focus, unlike the full harmonica
/// synth used elsewhere, which is deliberately breathy/textured.
pub(super) fn synth_reference_tone(freq: f32) -> Vec<u8> {
    const SAMPLE_RATE: u32 = 44_100;
    const DUR_SECS: f32 = 1.1;
    let n = (SAMPLE_RATE as f32 * DUR_SECS) as usize;
    let mut buf = vec![0.0f32; n];
    for (i, sample) in buf.iter_mut().enumerate() {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.02).min(1.0);
        let release = ((DUR_SECS - t) / 0.15).clamp(0.0, 1.0);
        let env = attack.min(release);
        let tau = std::f32::consts::TAU;
        let s = (tau * freq * t).sin()
            + 0.30 * (tau * freq * 2.0 * t).sin()
            + 0.12 * (tau * freq * 3.0 * t).sin();
        *sample = env * 0.28 * s;
    }
    encode_wav(&buf, SAMPLE_RATE)
}

/// Drives the adaptive drill while it's on: holds the current target until
/// the player sustains it in tune, credits a hit, and picks the next target,
/// weighted toward whatever's been missed most. A target the player can't
/// land within [`DRILL_TIMEOUT_SECS`] is scored a miss so the drill keeps
/// moving instead of stalling on one hole.
pub fn drill_update(
    key: Res<TrainerKey>,
    mut target: ResMut<TrainerTarget>,
    active: Res<ActivePitches>,
    trace: Res<BendTrace>,
    mut drill: ResMut<DrillState>,
    time: Res<Time>,
) {
    if !drill.enabled {
        return;
    }
    let harp = richter_harp(&key.0);
    if target_note(&harp, *target).is_none() {
        return;
    }
    let dt = time.delta_secs();
    drill.elapsed_secs += dt;

    let in_tune = !trace.unstable
        && matches!(
            tuner_observation(&harp, *target, &active),
            Some(TunerObservation::TargetFamily(cents)) if cents.abs() <= IN_TUNE_CENTS
        );
    drill.hold_secs = if in_tune { drill.hold_secs + dt } else { 0.0 };

    let hit = drill.hold_secs >= DRILL_HOLD_TO_ADVANCE;
    let timed_out = drill.elapsed_secs >= DRILL_TIMEOUT_SECS;
    if !hit && !timed_out {
        return;
    }

    let stat = drill
        .stats
        .entry((target.hole, target.technique))
        .or_default();
    stat.attempts += 1;
    if hit {
        stat.hits += 1;
        drill.streak += 1;
    } else {
        drill.streak = 0;
    }

    if let Some(next) = pick_next_target(&harp, &drill.stats, Some(*target)) {
        *target = next;
    }
    drill.hold_secs = 0.0;
    drill.elapsed_secs = 0.0;
}

/// Persists the session's drill hit-rates to `profile.json` on the way out
/// of the trainer — paired with `setup`'s restore on the way in. Saved once
/// per visit rather than on every drill-target completion, the same
/// "meaningful lifecycle boundary" policy `results::setup` uses for song
/// bests (see `profile.rs`'s module doc comment).
pub fn save_drill_progress(drill: Res<DrillState>, mut profile: ResMut<PlayerProfile>) {
    profile.drills = stats_to_profile(&drill.stats);
    harmonicon_app::profile::save_profile(&profile);
}

/// Keeps the "Drill: ..." readout in step with on/off state and streak.
pub fn update_drill_label(
    drill: Res<DrillState>,
    loc: Res<Localization>,
    mut labels: Query<&mut Text, With<DrillLabel>>,
) {
    if !drill.is_changed() {
        return;
    }
    for mut text in &mut labels {
        *text = Text::new(String::from(if drill.enabled {
            loc.msg_args("bending-drill-on", &[("streak", drill.streak.to_string())])
        } else {
            loc.msg("bending-drill-off")
        }));
    }
}

/// A plain dark amber, distinct from every other button's resting
/// [`button::color_default`], for the Drill button while the drill is
/// running.
pub(super) const DRILL_ACTIVE_COLOR: Color = Color::srgb(0.45, 0.35, 0.10);

/// Highlights the Drill toggle button itself while the drill is running,
/// alongside [`update_drill_label`]'s text — a toggle should visibly look
/// pressed, not only say so in small text next to it. Writes
/// [`BaseButtonColor`], not `BackgroundColor` directly: `dialogs::button`'s
/// own hover/press layer owns `BackgroundColor` and would fight a system
/// that wrote it here (see `BaseButtonColor`'s doc comment).
pub fn update_drill_button_visual(
    drill: Res<DrillState>,
    mut buttons: Query<&mut BaseButtonColor, With<DrillToggleButton>>,
) {
    if !drill.is_changed() {
        return;
    }
    let color = if drill.enabled {
        DRILL_ACTIVE_COLOR
    } else {
        button::color_default()
    };
    for mut base in &mut buttons {
        base.0 = color;
    }
}
