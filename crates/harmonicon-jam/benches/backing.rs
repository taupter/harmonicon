// SPDX-License-Identifier: MIT

//! What the generated Jam band costs to render. All three renders run
//! synchronously on the main thread:
//!
//! - `generate_backing_stems`: the whole backing (bass, drums, comping for
//!   every chorus), rendered when Start is clicked on the Jam generator
//!   page (`build_generated_manifest`).
//! - `render_band_answer`: a two-beat drum fill or chord push, rendered
//!   mid-session at a phrase end (`jam::band`).
//! - `generate_ending_stems`: the final tonic hit, rendered when the
//!   session ends.
//!
//! The call-and-response call is `harmonicon_core::synth::render_pcm` on
//! about eight notes; `harmonicon-bench`'s `core_hot_paths` measures that.
//!
//! Run with `cargo bench -p harmonicon-jam --bench backing`.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use harmonicon_core::harmonica::{ChordQuality, Progression};
use harmonicon_jam::jam::backing::{
    BandAnswer, BandEnergy, Genre, generate_backing_stems, generate_ending_stems,
    render_band_answer,
};

/// Swung and straight feels take different slot paths.
const GENRES: [(&str, Genre); 2] = [("blues", Genre::Blues), ("rock", Genre::Rock)];

fn backing_stems(c: &mut Criterion) {
    let mut group = c.benchmark_group("generate_backing_stems");
    // Each run renders minutes of audio; ten samples keep the bench short.
    group.sample_size(10);
    for (label, genre) in GENRES {
        for bpm in [80.0f32, 140.0] {
            group.bench_function(BenchmarkId::new(label, bpm), |b| {
                b.iter(|| {
                    black_box(generate_backing_stems(
                        "C",
                        black_box(bpm),
                        Progression::Standard,
                        genre,
                        BandEnergy::Medium,
                        42,
                    ))
                });
            });
        }
    }
    group.finish();
}

fn band_answer(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_band_answer");
    for (label, genre) in GENRES {
        for (answer_label, answer) in [("drums", BandAnswer::Drums), ("chord", BandAnswer::Chord)] {
            group.bench_function(BenchmarkId::new(answer_label, label), |b| {
                b.iter(|| {
                    black_box(render_band_answer(
                        answer,
                        genre,
                        "C",
                        ChordQuality::Dominant7,
                        black_box(100.0),
                    ))
                });
            });
        }
    }
    group.finish();
}

fn ending_stems(c: &mut Criterion) {
    let mut group = c.benchmark_group("generate_ending_stems");
    for (label, genre) in GENRES {
        group.bench_function(label, |b| {
            b.iter(|| {
                black_box(generate_ending_stems(
                    "C",
                    ChordQuality::Dominant7,
                    black_box(100.0),
                    genre,
                ))
            });
        });
    }
    group.finish();
}

criterion_group!(benches, backing_stems, band_answer, ending_stems);
criterion_main!(benches);
