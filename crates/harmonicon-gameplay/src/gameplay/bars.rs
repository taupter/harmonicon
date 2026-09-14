// SPDX-License-Identifier: MIT

//! Bar-position math: which meter a chart is in, bar-index arithmetic, and
//! the per-frame [`CurrentBar`]/[`AbsoluteBar`] tracker shared by
//! `twelve_bar_blues_overlay` and `jam::session`.

use bevy::prelude::*;

use harmonicon_app::app::SelectedSong;
use harmonicon_core::chart::{HarpChart, time_sig_at_tick};
use harmonicon_song::song::SongManifest;
use harmonicon_ui::music_score::{MusicScoreMeter, parse_time_signature};

use super::clock::GameplayClock;
use super::state::ScoringConfig;

/// The meter a chart is in — **the one place gameplay reads it from.**
///
/// Every bar-length figure in gameplay, Jam Session and the metronome
/// derives from the `MusicScoreMeter` this returns, through its own
/// methods (`beats_per_bar` for quarter-note beats, `numerator` for the
/// meter's own, `bar_secs` for seconds). Before this there were four
/// independent readings of the same field — two took the numerator alone
/// and called it a beat count, one rounded to whole quarters, one assumed
/// 4/4 outright — and they disagreed with each other on the same chart:
/// in 6/8 the metronome accented every second bar while the ruler was
/// right, and in 3/8 the accent walked around the bar and never settled.
///
/// A `timing.time_signature_map` entry at tick 0 wins over the song-level
/// field, the same precedence `setup_scoring_config` always applied — the
/// metronome used to skip the map and could disagree with scoring on a
/// chart that had one.
pub fn chart_meter(chart: &HarpChart) -> MusicScoreMeter {
    let sig = chart
        .timing
        .time_signature_map
        .as_deref()
        .and_then(|m| time_sig_at_tick(0, m))
        .or(chart.song.time_signature.as_deref())
        .unwrap_or("4/4");
    parse_time_signature(sig)
}

/// How many whole bars have elapsed since the clock last hit 0 (song/jam
/// start, or a loop rewind) — unlike [`current_bar_index`], not wrapped to
/// the 12-bar cycle.
pub fn absolute_bar_index(clock: f64, secs_per_bar: f64) -> usize {
    (clock.max(0.0) / secs_per_bar) as usize
}

/// Which of the 12 bars in a twelve-bar cycle the clock is currently on.
pub fn current_bar_index(clock: f64, secs_per_bar: f64) -> usize {
    absolute_bar_index(clock, secs_per_bar) % 12
}

/// The bar `track_current_bar` last computed — shared so
/// `twelve_bar_blues_overlay::update_bar` and `jam::session::
/// update_hole_map` don't each recompute it from two different
/// beats-per-bar sources that could disagree (`ScoringConfig::
/// beats_per_bar`, which honors a chart's `time_signature_map` override,
/// vs `JamHoleGuide`'s own copy).
#[derive(Resource, Default)]
pub struct CurrentBar(pub usize);

/// [`absolute_bar_index`]'s result, tracked the same frame as [`CurrentBar`]
/// — `jam::improv`'s phrase-discipline lesson primitive needs a play/rest
/// bar pattern that repeats consistently across an open-ended jam, not one
/// that resets every 12 bars the way `CurrentBar` does.
#[derive(Resource, Default)]
pub struct AbsoluteBar(pub usize);

/// Emitted by [`track_current_bar`] whenever the current bar changes,
/// forward or (on a loop rewind) backward — lets `update_bar` recolor the
/// 12-bar grid only on an actual change instead of writing `BackgroundColor`
/// on all 12 cells every frame. `update_hole_map` doesn't need this: it
/// repaints every frame anyway for live mic feedback.
#[derive(Message)]
pub struct BarChanged(pub usize);

/// Computes the current bar once per frame (must run after `clock::
/// handle_loop_boundary` so a loop rewind is reflected the same frame) and
/// emits [`BarChanged`] on a change, detected by recomputing from the clock
/// each frame rather than an incrementing counter — the same trick
/// `phrase_overlay::watch_phrase_boundaries` uses so a backward jump needs
/// no special-case handling.
pub(crate) fn track_current_bar(
    clock: Res<GameplayClock>,
    selected: Res<SelectedSong>,
    manifests: Res<Assets<SongManifest>>,
    config: Res<ScoringConfig>,
    mut current: ResMut<CurrentBar>,
    mut absolute: ResMut<AbsoluteBar>,
    mut last: Local<Option<usize>>,
    mut changed: MessageWriter<BarChanged>,
) {
    let Some(manifest) = manifests.get(&selected.0) else {
        return;
    };
    let bpm = manifest.chart.song.tempo_bpm as f64;
    let spb = config.meter.bar_secs(bpm);
    let bar = current_bar_index(clock.get(), spb);
    current.0 = bar;
    absolute.0 = absolute_bar_index(clock.get(), spb);
    if *last != Some(bar) {
        changed.write(BarChanged(bar));
    }
    *last = Some(bar);
}
