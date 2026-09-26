// SPDX-License-Identifier: MIT

//! A chart's lyrics as karaoke lines: which syllables make up each line, and
//! how far through them the singer is at a given moment.
//!
//! Lyrics ride on the notes: a track item's `lyric` is the syllable sung
//! when that item starts. Two conventions in the text shape the lines, the
//! same ones sheet music and karaoke files use:
//!
//! - a trailing `-` joins a syllable to the next one of the same word
//!   ("A-", "maz-", "ing" reads "Amazing");
//! - a leading `/` starts a new line. A new phrase tag on an item starts one
//!   too, since a phrase is already where the tune breathes.
//!
//! A harmonica part has more notes than a singer has syllables, so an item
//! without a lyric just extends the syllable before it.

use crate::chart::{HarpChart, TrackItem, tick_to_seconds};

/// One sung syllable, as it is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Syllable {
    /// The text without its `-` and `/` markers.
    pub text: String,
    /// When its note starts, in chart seconds.
    pub start: f64,
    /// Whether the next syllable continues the same word, so no space
    /// follows this one.
    pub joins_next: bool,
}

/// One line of lyrics, shown at once.
#[derive(Debug, Clone, PartialEq)]
pub struct LyricLine {
    pub syllables: Vec<Syllable>,
}

impl LyricLine {
    /// When its first syllable is sung.
    pub fn start(&self) -> f64 {
        self.syllables.first().map_or(0.0, |s| s.start)
    }

    /// The line as one string, words spaced and joined syllables not.
    pub fn text(&self) -> String {
        self.syllables
            .iter()
            .map(|s| {
                if s.joins_next {
                    s.text.clone()
                } else {
                    format!("{} ", s.text)
                }
            })
            .collect::<String>()
            .trim_end()
            .to_string()
    }
}

fn item_seconds(item: &TrackItem, chart: &HarpChart) -> f64 {
    item.time.unwrap_or_else(|| {
        tick_to_seconds(
            item.tick.unwrap_or(0),
            chart.timing.resolution,
            &chart.timing.tempo_map,
        )
    })
}

/// Every line of `chart`'s lyrics, in time order. Empty when nothing has a
/// lyric, which is most charts.
pub fn lyric_lines(chart: &HarpChart) -> Vec<LyricLine> {
    let mut items: Vec<(f64, &TrackItem)> = chart
        .track
        .iter()
        .map(|item| (item_seconds(item, chart), item))
        .collect();
    items.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut lines: Vec<LyricLine> = Vec::new();
    let mut current: Vec<Syllable> = Vec::new();
    let mut phrase: Option<&str> = None;
    for (start, item) in items {
        let new_phrase = item
            .phrase
            .as_deref()
            .filter(|p| !p.is_empty() && Some(*p) != phrase);
        if let Some(p) = new_phrase {
            phrase = Some(p);
            if !current.is_empty() {
                lines.push(LyricLine {
                    syllables: std::mem::take(&mut current),
                });
            }
        }
        let Some(raw) = item.lyric.as_deref().map(str::trim) else {
            continue;
        };
        let (breaks, raw) = match raw.strip_prefix('/') {
            Some(rest) => (true, rest.trim_start()),
            None => (false, raw),
        };
        if breaks && !current.is_empty() {
            lines.push(LyricLine {
                syllables: std::mem::take(&mut current),
            });
        }
        let (joins_next, text) = match raw.strip_suffix('-') {
            Some(rest) => (true, rest),
            None => (false, raw),
        };
        if text.is_empty() {
            continue;
        }
        current.push(Syllable {
            text: text.to_string(),
            start,
            joins_next,
        });
    }
    if !current.is_empty() {
        lines.push(LyricLine { syllables: current });
    }
    // A word can't join across a line break.
    for line in &mut lines {
        if let Some(last) = line.syllables.last_mut() {
            last.joins_next = false;
        }
    }
    lines
}

/// Where the singer is at one moment: the line on screen and how many of
/// its syllables have started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KaraokePosition {
    pub line: usize,
    pub sung: usize,
}

/// The karaoke position at `clock` seconds. A line takes over when its
/// first syllable is sung — the line after it is shown below, so it can be
/// read ahead — and before the first one the opening line waits unsung.
/// `None` without lyrics.
pub fn karaoke_at(lines: &[LyricLine], clock: f64) -> Option<KaraokePosition> {
    if lines.is_empty() {
        return None;
    }
    let line = lines
        .partition_point(|l| l.start() <= clock)
        .saturating_sub(1);
    let sung = lines[line].syllables.partition_point(|s| s.start <= clock);
    Some(KaraokePosition { line, sung })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chart(items: &[(f64, Option<&str>, Option<&str>)]) -> HarpChart {
        let mut chart: HarpChart = serde_json::from_str(
            r#"{
                "song": { "title": "T", "artist": "A", "tempo_bpm": 120.0,
                          "key": "C", "difficulty": "easy" },
                "timing": { "resolution": 12, "tempo_map": [{"tick": 0, "bpm": 120.0}] },
                "harmonica": {
                    "type": "diatonic", "holes": 10,
                    "bending_profile": "richter_standard",
                    "layout": {
                        "blow": ["C4","E4","G4","C5","E5","G5","C6","E6","G6","C7"],
                        "draw": ["D4","G4","B4","D5","F5","A5","B5","D6","F6","A6"]
                    }
                },
                "track": [],
                "scoring": { "perfect_window_ms": 50, "good_window_ms": 100,
                             "miss_window_ms": 130 }
            }"#,
        )
        .unwrap();
        chart.track = items
            .iter()
            .map(|&(time, lyric, phrase)| {
                serde_json::from_value(serde_json::json!({
                    "time": time, "duration": 0.25, "lyric": lyric, "phrase": phrase,
                    "events": [{ "hole": 4, "action": "blow" }]
                }))
                .unwrap()
            })
            .collect();
        chart
    }

    fn texts(lines: &[LyricLine]) -> Vec<String> {
        lines.iter().map(LyricLine::text).collect()
    }

    #[test]
    fn hyphens_join_syllables_into_words() {
        let c = chart(&[
            (0.0, Some("A-"), None),
            (0.5, Some("maz-"), None),
            (1.0, Some("ing"), None),
            (1.5, Some("grace"), None),
        ]);
        assert_eq!(texts(&lyric_lines(&c)), vec!["Amazing grace"]);
    }

    #[test]
    fn a_slash_or_a_new_phrase_starts_a_line() {
        let c = chart(&[
            (0.0, Some("how"), Some("verse")),
            (0.5, Some("sweet"), None),
            (1.0, Some("/the"), None),
            (1.5, Some("sound"), None),
            (2.0, Some("that"), Some("chorus")),
        ]);
        assert_eq!(
            texts(&lyric_lines(&c)),
            vec!["how sweet", "the sound", "that"]
        );
    }

    #[test]
    fn notes_without_a_lyric_extend_the_syllable_before() {
        let c = chart(&[
            (0.0, Some("grace"), None),
            (0.5, None, None),
            (1.0, None, None),
            (1.5, Some("that"), None),
        ]);
        let lines = lyric_lines(&c);
        assert_eq!(lines[0].syllables.len(), 2);
        assert_eq!(lines[0].syllables[1].start, 1.5);
    }

    #[test]
    fn items_out_of_order_are_sung_in_time_order() {
        let c = chart(&[(1.0, Some("world"), None), (0.0, Some("hello"), None)]);
        assert_eq!(texts(&lyric_lines(&c)), vec!["hello world"]);
    }

    #[test]
    fn a_word_does_not_join_across_a_line_break() {
        let c = chart(&[(0.0, Some("broken-"), None), (1.0, Some("/line"), None)]);
        let lines = lyric_lines(&c);
        assert_eq!(texts(&lines), vec!["broken", "line"]);
        assert!(!lines[0].syllables[0].joins_next);
    }

    #[test]
    fn a_chart_without_lyrics_has_no_lines() {
        let c = chart(&[(0.0, None, None), (1.0, Some("  "), None)]);
        assert!(lyric_lines(&c).is_empty());
        assert_eq!(karaoke_at(&[], 3.0), None);
    }

    #[test]
    fn karaoke_follows_the_clock_syllable_by_syllable() {
        let c = chart(&[
            (1.0, Some("one"), None),
            (2.0, Some("two"), None),
            (3.0, Some("/three"), None),
            (4.0, Some("four"), None),
        ]);
        let lines = lyric_lines(&c);
        let at = |clock| karaoke_at(&lines, clock).unwrap();
        // Before the song: the first line waits, nothing sung.
        assert_eq!(at(-2.0), KaraokePosition { line: 0, sung: 0 });
        assert_eq!(at(1.0), KaraokePosition { line: 0, sung: 1 });
        assert_eq!(at(2.5), KaraokePosition { line: 0, sung: 2 });
        // The second line takes over on its first syllable.
        assert_eq!(at(3.0), KaraokePosition { line: 1, sung: 1 });
        assert_eq!(at(99.0), KaraokePosition { line: 1, sung: 2 });
    }
}
