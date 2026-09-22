// SPDX-License-Identifier: MIT

//! Exports short, repeatable generated-band mixes for subjective review.

use std::error::Error;
use std::path::{Path, PathBuf};

use harmonicon_core::harmonica::Progression;
use harmonicon_core::wav::encode_wav;
use harmonicon_jam::jam::backing::{BandEnergy, Genre, SAMPLE_RATE, generate_listening_preview};

const REVIEW_SEED: u64 = 0x4841_524d_4f4e_4943;

fn slug(genre: Genre) -> &'static str {
    match genre {
        Genre::Blues => "blues",
        Genre::Jazz => "jazz",
        Genre::Rock => "rock",
        Genre::Reggae => "reggae",
        Genre::Country => "country",
    }
}

fn write_preview(output: &Path, genre: Genre, bpm: f32) -> Result<PathBuf, Box<dyn Error>> {
    let pcm = generate_listening_preview(
        "C",
        bpm,
        Progression::Standard,
        genre,
        BandEnergy::Medium,
        REVIEW_SEED,
    );
    let path = output.join(format!("{}-{bpm:.0}bpm.wav", slug(genre)));
    std::fs::write(&path, encode_wav(&pcm, SAMPLE_RATE))?;
    Ok(path)
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/jam-listening"));
    std::fs::create_dir_all(&output)?;

    for genre in [Genre::Blues, Genre::Jazz, Genre::Reggae] {
        for bpm in [70.0, 100.0, 140.0] {
            println!("{}", write_preview(&output, genre, bpm)?.display());
        }
    }
    Ok(())
}
