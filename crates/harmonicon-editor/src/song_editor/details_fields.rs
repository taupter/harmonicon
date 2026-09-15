// SPDX-License-Identifier: MIT

//! Registry of song-level rows in the Details form. The field behavior and
//! backing values remain in `state`/`meta_form`; this module only owns their
//! display order and localization keys.

use super::state::Field;

pub(super) const FIELDS: [(Field, &str); 19] = [
    (Field::Tempo, "editor-field-tempo"),
    (Field::Key, "editor-field-key"),
    (Field::Position, "editor-field-position"),
    (Field::Music, "editor-field-music"),
    (Field::Name, "editor-field-name"),
    (Field::Author, "editor-field-author"),
    (Field::Difficulty, "editor-field-difficulty"),
    (Field::SongFeel, "editor-field-feel"),
    (Field::Source, "editor-field-source"),
    (Field::License, "editor-field-license"),
    (Field::Description, "editor-field-description"),
    (Field::PerfectWindow, "editor-field-perfect-window"),
    (Field::GoodWindow, "editor-field-good-window"),
    (Field::MissWindow, "editor-field-miss-window"),
    (Field::ComboEnabled, "editor-field-combo-enabled"),
    (Field::ComboBase, "editor-field-combo-base"),
    (Field::ComboStep, "editor-field-combo-step"),
    (Field::ComboMax, "editor-field-combo-max"),
    (Field::ComboDecay, "editor-field-combo-decay"),
];
