// SPDX-License-Identifier: MIT

//! Selected-note phrase labels and expression depth editing.

use super::state::{EditorState, Expr, Field};

impl EditorState {
    pub(super) fn selected_annotation_text(&self, field: Field) -> &str {
        let Some(tick) = self.selected_note().map(|note| note.tick) else {
            return "";
        };
        let Some(annotation) = self.phrase_annotations.get(&tick) else {
            return "";
        };
        match field {
            Field::Section => annotation.section.as_deref().unwrap_or(""),
            Field::Chord => annotation.chord.as_deref().unwrap_or(""),
            Field::Groove => annotation.groove.as_deref().unwrap_or(""),
            _ => unreachable!(),
        }
    }

    pub(super) fn selected_expression_intensity(&self) -> &str {
        let Some(note) = self.selected_note() else {
            return "";
        };
        if note.expr == Expr::None {
            return "";
        }
        self.expression_intensities
            .get(&note.id)
            .map(String::as_str)
            .unwrap_or("0.5")
    }

    pub(super) fn set_selected_expression_intensity(&mut self, value: String) {
        let Some(note) = self.selected_note() else {
            return;
        };
        if note.expr == Expr::None {
            return;
        }
        let id = note.id;
        let value = value.trim();
        if value.is_empty() || value == "0.5" {
            self.expression_intensities.remove(&id);
        } else if value.parse::<f32>().is_ok_and(|v| (0.0..=1.0).contains(&v)) {
            self.expression_intensities.insert(id, value.to_owned());
        }
    }

    pub(super) fn set_selected_annotation(&mut self, field: Field, value: String) {
        if !matches!(field, Field::Section | Field::Chord | Field::Groove) {
            return;
        }
        let Some(tick) = self.selected_note().map(|note| note.tick) else {
            return;
        };
        let value = (!value.trim().is_empty()).then_some(value);
        let annotation = self.phrase_annotations.entry(tick).or_default();
        match field {
            Field::Section => annotation.section = value,
            Field::Chord => annotation.chord = value,
            Field::Groove => annotation.groove = value,
            _ => unreachable!(),
        }
        self.remove_empty_annotation(tick);
    }

    pub(super) fn selected_call(&self) -> bool {
        self.selected_note()
            .and_then(|n| self.phrase_annotations.get(&n.tick))
            .is_some_and(|annotation| annotation.call)
    }

    pub(super) fn set_selected_call(&mut self, call: bool) {
        let Some(tick) = self.selected_note().map(|note| note.tick) else {
            return;
        };
        if call {
            self.phrase_annotations.entry(tick).or_default().call = true;
        } else if let Some(annotation) = self.phrase_annotations.get_mut(&tick) {
            annotation.call = false;
            self.remove_empty_annotation(tick);
        }
    }

    pub(super) fn selected_split(&self) -> bool {
        self.selected_note()
            .and_then(|n| self.phrase_annotations.get(&n.tick))
            .is_some_and(|annotation| annotation.split)
    }

    pub(super) fn set_selected_split(&mut self, split: bool) {
        let Some(tick) = self.selected_note().map(|note| note.tick) else {
            return;
        };
        if split {
            self.phrase_annotations.entry(tick).or_default().split = true;
        } else if let Some(annotation) = self.phrase_annotations.get_mut(&tick) {
            annotation.split = false;
            self.remove_empty_annotation(tick);
        }
    }

    fn remove_empty_annotation(&mut self, tick: usize) {
        if self
            .phrase_annotations
            .get(&tick)
            .is_some_and(|annotation| {
                annotation.section.is_none()
                    && annotation.chord.is_none()
                    && annotation.groove.is_none()
                    && !annotation.call
                    && !annotation.split
            })
        {
            self.phrase_annotations.remove(&tick);
        }
    }
}
