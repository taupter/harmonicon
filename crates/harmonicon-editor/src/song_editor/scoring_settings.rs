// SPDX-License-Identifier: MIT

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ComboSettings {
    pub(super) enabled: String,
    pub(super) base: String,
    pub(super) step: String,
    pub(super) max: String,
    pub(super) decay_ms: String,
}

impl Default for ComboSettings {
    fn default() -> Self {
        Self {
            enabled: "enabled".into(),
            base: "1".into(),
            step: "0.1".into(),
            max: "4".into(),
            decay_ms: "2000".into(),
        }
    }
}
