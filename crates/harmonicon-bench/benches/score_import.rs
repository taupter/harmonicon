// SPDX-License-Identifier: MIT

//! MIDI import stages on bundled files: initial parse, extraction of a
//! playable track's notes, and conversion of all tracks to harmonica charts.
//! These run when a player opens an imported score. Conversion includes the
//! repeated MIDI reads and best-harp search that the real loader performs.
//!
//! Run with `cargo bench -p harmonicon-bench --bench score_import`.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use harmonicon_score::convert::convert_all_tracks;
use harmonicon_score::{ScoreFile, midi::MidiScore};

const SMALL: &[u8] = include_bytes!("../../../assets/midi/AUD_CT0449.mid");
const LARGE: &[u8] = include_bytes!("../../../assets/midi/AUD_AP6341.mid");

fn score_import(c: &mut Criterion) {
    let mut group = c.benchmark_group("score_import");
    for (label, bytes) in [("small", SMALL), ("large", LARGE)] {
        let score = MidiScore::parse(bytes.to_vec()).expect("bundled MIDI parses");
        let track = score
            .tracks()
            .iter()
            .filter(|track| track.is_playable())
            .max_by_key(|track| track.note_count)
            .expect("bundled MIDI has notes");
        let track_index = track.index;
        group.bench_with_input(BenchmarkId::new("parse", label), &bytes, |b, bytes| {
            b.iter_batched(
                || bytes.to_vec(),
                |input| black_box(MidiScore::parse(black_box(input)).unwrap()),
                BatchSize::SmallInput,
            );
        });
        group.bench_function(BenchmarkId::new("notes", label), |b| {
            b.iter(|| black_box(score.notes(black_box(track_index)).unwrap()));
        });
        group.bench_function(BenchmarkId::new("convert_all_tracks", label), |b| {
            b.iter(|| black_box(convert_all_tracks(black_box(&score), "Benchmark").unwrap()));
        });
    }
    group.finish();
}

criterion_group!(benches, score_import);
criterion_main!(benches);
