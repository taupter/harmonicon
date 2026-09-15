// SPDX-License-Identifier: MIT

//! Wires the Song Editor's own note/playhead state into the shared
//! `music_score` overlay — the editor's sibling of `gameplay::
//! music_score_bridge`, translating `EditorState::notes`/`playback::
//! Playhead` into `MusicScoreNotes`/`MusicScorePlayhead`. Editor ticks are
//! already tempo-independent multiples of a beat (`TICKS_PER_BEAT`), so
//! unlike gameplay's bridge this needs no tempo-map conversion, just a
//! division.

use bevy::prelude::*;

use harmonicon_ui::music_score::MusicScoreMeter;
use harmonicon_ui::music_score::{
    MeterMap, MusicScoreNotes, MusicScorePlayhead, NotationNote, parse_time_signature,
};

use super::TICKS_PER_BEAT;
use super::playback::{Playhead, note_midi};
use super::state::EditorState;

fn notation_segments(
    note: &super::state::GridNote,
    midi: u8,
    meter_map: &MeterMap,
) -> Vec<NotationNote> {
    let start = note.tick as u64;
    let end = (note.tick + note.len.max(1)) as u64;
    let mut boundaries = vec![start];
    boundaries.extend(
        meter_map
            .bar_starts(start.saturating_add(1), end)
            .into_iter()
            .map(|(tick, _)| tick),
    );
    boundaries.push(end);
    boundaries
        .windows(2)
        .enumerate()
        .map(|(index, pair)| NotationNote {
            start_beat: pair[0] as f64 / TICKS_PER_BEAT as f64,
            duration_beats: (pair[1] - pair[0]) as f64 / TICKS_PER_BEAT as f64,
            midi,
            tied_from_previous: index > 0,
        })
        .collect()
}

/// Rebuilds [`MusicScoreNotes`] from `EditorState::notes` whenever the
/// editor state changes — same `resource_exists_and_changed::<EditorState>`
/// gate every other EditorState-derived rebuild in `song_editor::mod` uses.
/// A note whose hole/technique the current harp can't resolve is skipped,
/// same as gameplay's bridge. The editor's meter map supplies every bar
/// boundary, so notes crossing either an ordinary bar line or a meter change
/// become tied segments instead of one oversized notehead.
pub(super) fn sync_music_score(
    state: Res<EditorState>,
    mut notes: ResMut<MusicScoreNotes>,
    mut meter: ResMut<MusicScoreMeter>,
) {
    let editor_meter = parse_time_signature(&state.time_signature);
    if *meter != editor_meter {
        *meter = editor_meter;
    }
    let harp = state.effective_harp();
    let meter_map = state.meter_map();
    notes.0 = state
        .notes
        .iter()
        .filter_map(|n| {
            let midi = note_midi(n, &harp)?;
            Some(notation_segments(n, midi, &meter_map))
        })
        .flatten()
        .collect();
}

/// Keeps [`MusicScorePlayhead`] following the same tick position
/// `playback::update_playhead_view`'s moving line derives from [`Playhead`]
/// while a take is playing (or paused mid-take) — ordered `.after(playback
/// ::advance_playhead)` so it reads the same frame's `elapsed`. Otherwise
/// (just editing, nothing running) it follows `EditorState::scroll_beat`
/// instead, so a note far from the song's start can still scroll into the
/// panel's fixed-width visible window rather than the score staying pinned
/// at beat 0.
pub(super) fn sync_music_score_playhead(
    playhead: Res<Playhead>,
    state: Res<EditorState>,
    mut score_playhead: ResMut<MusicScorePlayhead>,
    mut meter: ResMut<MusicScoreMeter>,
) {
    let tick = if playhead.playing && playhead.secs_per_tick > 0.0 {
        let cur_tick = playhead.elapsed / playhead.secs_per_tick;
        score_playhead.0 = (cur_tick / TICKS_PER_BEAT as f32) as f64;
        cur_tick.max(0.0).round() as u64
    } else {
        score_playhead.0 = state.scroll_beat as f64;
        (state.scroll_beat * TICKS_PER_BEAT) as u64
    };
    *meter = state.meter_map().meter_at(tick);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song_editor::state::{Dir, Expr, GridNote, Pitch};

    fn note(tick: usize, len: usize) -> GridNote {
        GridNote {
            id: 1,
            hole: 4,
            tick,
            len,
            dir: Dir::Blow,
            pitch: Pitch::Normal,
            expr: Expr::None,
        }
    }

    #[test]
    fn notation_splits_and_ties_across_meter_map_bars() {
        let map = MeterMap::new([(0, "4/4"), (48, "3/4")], TICKS_PER_BEAT as u32);

        let segments = notation_segments(&note(36, 60), 60, &map);

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].duration_beats, 1.0);
        assert_eq!(segments[1].duration_beats, 3.0);
        assert_eq!(segments[2].duration_beats, 1.0);
        assert!(!segments[0].tied_from_previous);
        assert!(segments[1].tied_from_previous);
        assert!(segments[2].tied_from_previous);
    }

    #[test]
    fn notation_treats_an_off_bar_meter_change_as_a_boundary() {
        let map = MeterMap::new([(0, "4/4"), (42, "3/4")], TICKS_PER_BEAT as u32);

        let segments = notation_segments(&note(36, 54), 60, &map);

        let starts: Vec<_> = segments.iter().map(|segment| segment.start_beat).collect();
        assert_eq!(starts, vec![3.0, 3.5, 6.5]);
        assert!(
            segments[1..]
                .iter()
                .all(|segment| segment.tied_from_previous)
        );
    }
}
