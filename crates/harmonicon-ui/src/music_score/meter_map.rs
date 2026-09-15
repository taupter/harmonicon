// SPDX-License-Identifier: MIT

//! Where the bars fall when the meter changes mid-piece.
//!
//! A single [`MusicScoreMeter`] answers "how long is a bar"; a [`MeterMap`]
//! answers "which bar and beat is tick *T* in" for a chart whose
//! `timing.time_signature_map` switches meter part-way. Everything
//! bar-shaped — the Song Editor's ruler, bar lines and 12-bar tint, its
//! "bar.beat" readouts, the notation staff's bar splitting, the count-in —
//! should ask this rather than a lone meter, or it drifts from the change
//! onward.
//!
//! Bars are numbered continuously across changes. A change that lands
//! *between* bar lines of the meter before it cuts that bar short: the new
//! meter starts a fresh bar at the change tick, and the truncated bar
//! still counts as one. That is what every notation program does with a
//! meter change and what a musician expects — a 4/4 → 3/4 change three
//! quarters into a bar means "that bar was a 3/4 bar", not "the next 3/4
//! bar starts a quarter late".
//!
//! Pure and Bevy-free (like `notation.rs`), so it's unit-tested without a
//! world. Ticks are whatever resolution the caller uses — the map only
//! needs to know how many ticks a quarter note is.

use super::{MusicScoreMeter, parse_time_signature};

/// One stretch of constant meter: from `start_tick` until the next
/// segment's start (or forever).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeterSegment {
    pub start_tick: u64,
    pub meter: MusicScoreMeter,
    /// 0-based index of the bar that starts at `start_tick`.
    pub first_bar: usize,
    /// Ticks in one beat of `meter` — the unit its upper number counts.
    pub ticks_per_beat: u64,
    /// Ticks in one bar of `meter`.
    pub ticks_per_bar: u64,
}

/// A chart's meter over time. Always has at least one segment, at tick 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeterMap {
    segments: Vec<MeterSegment>,
}

/// Where a tick falls: 0-based bar and beat, and the tick offset into
/// that beat. Display code adds one to `bar`/`beat`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarPosition {
    pub bar: usize,
    pub beat: usize,
    pub tick_in_beat: u64,
    /// The meter in force at that tick.
    pub meter: MusicScoreMeter,
}

impl MeterMap {
    /// Builds the map from `(tick, "N/D")` points, `quarter_ticks` ticks to
    /// a quarter note. Points are sorted; a duplicate tick keeps the later
    /// entry; a point that doesn't cover tick 0 gets a 4/4 segment in front
    /// so every tick has a meter. A meter too fine for the tick grid (a
    /// beat that isn't a whole number of ticks) falls back to 4/4 rather
    /// than producing a zero-length bar the arithmetic would divide by.
    pub fn new<'a>(points: impl IntoIterator<Item = (u64, &'a str)>, quarter_ticks: u32) -> Self {
        let mut points: Vec<(u64, MusicScoreMeter)> = points
            .into_iter()
            .map(|(tick, sig)| (tick, parse_time_signature(sig)))
            .collect();
        points.sort_by_key(|(tick, _)| *tick);
        points.dedup_by(|later, earlier| {
            if later.0 == earlier.0 {
                earlier.1 = later.1;
                true
            } else {
                false
            }
        });
        if points.first().is_none_or(|(tick, _)| *tick != 0) {
            points.insert(0, (0, MusicScoreMeter::default()));
        }

        let mut segments: Vec<MeterSegment> = Vec::with_capacity(points.len());
        let mut next_bar = 0usize;
        for (i, &(start_tick, meter)) in points.iter().enumerate() {
            let (ticks_per_beat, ticks_per_bar) = match (
                meter.ticks_per_beat(quarter_ticks),
                meter.ticks_per_bar(quarter_ticks),
            ) {
                (Some(beat), Some(bar)) if beat > 0 && bar > 0 => (u64::from(beat), u64::from(bar)),
                _ => {
                    let fallback = MusicScoreMeter::default();
                    (
                        u64::from(fallback.ticks_per_beat(quarter_ticks).unwrap_or(1).max(1)),
                        u64::from(fallback.ticks_per_bar(quarter_ticks).unwrap_or(4).max(1)),
                    )
                }
            };
            segments.push(MeterSegment {
                start_tick,
                meter,
                first_bar: next_bar,
                ticks_per_beat,
                ticks_per_bar,
            });
            // Bars this segment contributes before the next change: full
            // bars, plus one for a change that cuts a bar short.
            if let Some(&(next_start, _)) = points.get(i + 1) {
                let span = next_start - start_tick;
                next_bar += span.div_ceil(ticks_per_bar) as usize;
            }
        }
        Self { segments }
    }

    /// A map with one meter for the whole piece.
    pub fn constant(meter: &str, quarter_ticks: u32) -> Self {
        Self::new([(0, meter)], quarter_ticks)
    }

    pub fn segments(&self) -> &[MeterSegment] {
        &self.segments
    }

    /// The segment in force at `tick`.
    pub fn segment_at(&self, tick: u64) -> &MeterSegment {
        let idx = self
            .segments
            .partition_point(|s| s.start_tick <= tick)
            .saturating_sub(1);
        &self.segments[idx]
    }

    pub fn meter_at(&self, tick: u64) -> MusicScoreMeter {
        self.segment_at(tick).meter
    }

    /// Which bar and beat `tick` is in.
    pub fn position(&self, tick: u64) -> BarPosition {
        let seg = self.segment_at(tick);
        let offset = tick - seg.start_tick;
        let bar_in_segment = (offset / seg.ticks_per_bar) as usize;
        let in_bar = offset % seg.ticks_per_bar;
        BarPosition {
            bar: seg.first_bar + bar_in_segment,
            beat: (in_bar / seg.ticks_per_beat) as usize,
            tick_in_beat: in_bar % seg.ticks_per_beat,
            meter: seg.meter,
        }
    }

    /// Whether a bar starts exactly at `tick`.
    pub fn is_bar_start(&self, tick: u64) -> bool {
        let seg = self.segment_at(tick);
        (tick - seg.start_tick).is_multiple_of(seg.ticks_per_bar)
    }

    /// The tick each *bar* starts on within `[from, to)`, with its 0-based
    /// index — for drawing bar lines and bar numbers. A bar cut short by a
    /// meter change still starts where it starts; the change's own tick is
    /// the next bar's start.
    pub fn bar_starts(&self, from: u64, to: u64) -> Vec<(u64, usize)> {
        let mut out = Vec::new();
        for (i, seg) in self.segments.iter().enumerate() {
            let seg_end = self.segments.get(i + 1).map_or(u64::MAX, |n| n.start_tick);
            if seg_end <= from || seg.start_tick >= to {
                continue;
            }
            // First bar start at or after `from` within this segment.
            let first = if from <= seg.start_tick {
                seg.start_tick
            } else {
                seg.start_tick
                    + (from - seg.start_tick).div_ceil(seg.ticks_per_bar) * seg.ticks_per_bar
            };
            let mut tick = first;
            while tick < to && tick < seg_end {
                let bar = seg.first_bar + ((tick - seg.start_tick) / seg.ticks_per_bar) as usize;
                out.push((tick, bar));
                tick += seg.ticks_per_bar;
            }
        }
        out
    }

    /// The tick each *beat* of the meter starts on within `[from, to)`,
    /// with its position — for the ruler's beat numbers. Like
    /// [`bar_starts`](Self::bar_starts), a beat cut short by a change is
    /// still listed at its own start.
    pub fn beat_starts(&self, from: u64, to: u64) -> Vec<(u64, BarPosition)> {
        let mut out = Vec::new();
        for (i, seg) in self.segments.iter().enumerate() {
            let seg_end = self.segments.get(i + 1).map_or(u64::MAX, |n| n.start_tick);
            if seg_end <= from || seg.start_tick >= to {
                continue;
            }
            let first = if from <= seg.start_tick {
                seg.start_tick
            } else {
                seg.start_tick
                    + (from - seg.start_tick).div_ceil(seg.ticks_per_beat) * seg.ticks_per_beat
            };
            let mut tick = first;
            while tick < to && tick < seg_end {
                out.push((tick, self.position(tick)));
                tick += seg.ticks_per_beat;
            }
        }
        out
    }

    /// Every meter *change* (every segment after the first) as
    /// `(tick, meter)` — for drawing change markers.
    pub fn changes(&self) -> impl Iterator<Item = (u64, MusicScoreMeter)> + '_ {
        self.segments
            .iter()
            .skip(1)
            .map(|s| (s.start_tick, s.meter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Q: u32 = 12;

    fn sig(m: MusicScoreMeter) -> String {
        format!("{}/{}", m.numerator, m.denominator)
    }

    #[test]
    fn a_constant_map_counts_bars_like_a_single_meter() {
        let map = MeterMap::constant("4/4", Q);
        assert_eq!(map.segments().len(), 1);
        let p = map.position(48 * 3 + 12 * 2 + 5);
        assert_eq!((p.bar, p.beat, p.tick_in_beat), (3, 2, 5));
        assert_eq!(sig(p.meter), "4/4");
    }

    #[test]
    fn bars_keep_counting_across_a_change_on_a_bar_line() {
        // Two bars of 4/4 (96 ticks), then 3/4.
        let map = MeterMap::new([(0, "4/4"), (96, "3/4")], Q);
        assert_eq!(map.position(95).bar, 1);
        assert_eq!(map.position(96).bar, 2, "the change starts bar 3");
        assert_eq!(map.position(96 + 36).bar, 3, "a 3/4 bar is 36 ticks");
        assert_eq!(map.position(96 + 36 + 24).beat, 2);
        assert_eq!(sig(map.meter_at(96)), "3/4");
        assert_eq!(sig(map.meter_at(95)), "4/4");
    }

    #[test]
    fn a_change_away_from_a_bar_line_cuts_that_bar_short() {
        // 4/4, changed to 3/4 three quarters into bar 2 (tick 48 + 36 = 84).
        let map = MeterMap::new([(0, "4/4"), (84, "3/4")], Q);
        // Bar 2 (index 1) runs 48..84 and is only 36 ticks long.
        assert_eq!(map.position(83).bar, 1);
        assert_eq!(map.position(83).beat, 2);
        // The change starts bar 3 immediately, not a quarter later.
        assert_eq!(map.position(84).bar, 2);
        assert_eq!(map.position(84).beat, 0);
        assert!(map.is_bar_start(84));
        assert!(!map.is_bar_start(96), "the old 4/4 grid no longer applies");
        assert!(map.is_bar_start(84 + 36));
    }

    #[test]
    fn six_eight_to_seven_eight_counts_in_eighths_on_both_sides() {
        // One bar of 6/8 (36 ticks), then 7/8 (42 ticks).
        let map = MeterMap::new([(0, "6/8"), (36, "7/8")], Q);
        assert_eq!(map.position(30).beat, 5, "sixth eighth of the 6/8 bar");
        assert_eq!(map.position(36).bar, 1);
        assert_eq!(
            map.position(36 + 6 * 6).beat,
            6,
            "seventh eighth of the 7/8 bar"
        );
        assert_eq!(map.position(36 + 42).bar, 2);
    }

    #[test]
    fn bar_starts_lists_every_bar_line_including_the_change_tick() {
        let map = MeterMap::new([(0, "4/4"), (84, "3/4")], Q);
        assert_eq!(
            map.bar_starts(0, 200),
            vec![(0, 0), (48, 1), (84, 2), (120, 3), (156, 4), (192, 5)]
        );
        // A window that starts mid-bar picks up from the next line.
        assert_eq!(map.bar_starts(50, 130), vec![(84, 2), (120, 3)]);
    }

    #[test]
    fn beat_starts_switches_beat_unit_at_the_change() {
        // 4/4 (quarter beats, 12 ticks) then 6/8 (eighth beats, 6 ticks).
        let map = MeterMap::new([(0, "4/4"), (48, "6/8")], Q);
        let ticks: Vec<u64> = map
            .beat_starts(36, 72)
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        assert_eq!(ticks, vec![36, 48, 54, 60, 66]);
        let (_, p) = map.beat_starts(54, 55)[0];
        assert_eq!((p.bar, p.beat), (1, 1));
    }

    #[test]
    fn a_map_without_a_tick_zero_point_gets_common_time_in_front() {
        let map = MeterMap::new([(48, "3/4")], Q);
        assert_eq!(sig(map.meter_at(0)), "4/4");
        assert_eq!(sig(map.meter_at(48)), "3/4");
        assert_eq!(map.position(48).bar, 1);
    }

    #[test]
    fn duplicate_ticks_keep_the_later_point_and_order_is_irrelevant() {
        let map = MeterMap::new([(96, "3/4"), (0, "4/4"), (96, "5/4")], Q);
        assert_eq!(sig(map.meter_at(96)), "5/4");
        assert_eq!(map.segments().len(), 2);
    }

    #[test]
    fn changes_lists_everything_after_the_opening_meter() {
        let map = MeterMap::new([(0, "4/4"), (96, "3/4"), (180, "6/8")], Q);
        let changes: Vec<(u64, String)> = map.changes().map(|(t, m)| (t, sig(m))).collect();
        assert_eq!(changes, vec![(96, "3/4".into()), (180, "6/8".into())]);
    }

    #[test]
    fn a_meter_too_fine_for_the_grid_falls_back_rather_than_dividing_by_zero() {
        let map = MeterMap::new([(0, "4/32")], Q);
        assert_eq!(map.segment_at(0).ticks_per_bar, 48);
        let _ = map.position(1000);
    }
}
