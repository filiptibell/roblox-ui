#![allow(dead_code)]

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum PaletteId {
    Platinum,
    Graphite,
    White,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum PaletteColor {
    Red,
    Yellow,
    Green,
    Blue,
    Purple,
    Grey,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct Palettes {
    pub defaults: HashMap<String, PaletteId>,
    pub palettes: Vec<Palette>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct Palette {
    pub id: PaletteId,
    pub title: String,
    #[serde(alias = "page_colours")]
    pub page_colors: PaletteId,
    #[serde(alias = "colours")]
    pub colors: HashMap<PaletteColor, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(super) struct IconData {
    pub icon: usize,
    pub name: String,
    pub glyph: String,
    #[serde(alias = "colour")]
    pub color: PaletteColor,
}
