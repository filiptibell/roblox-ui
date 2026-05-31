#![allow(dead_code)]

use super::*;

pub(super) const PALETTE_ID_SOURCE: PaletteId = PaletteId::Platinum;

pub(super) const PACK_CONTENTS_ICON_DATA: &[u8] = include_bytes!("./archive/icondata.json");
pub(super) const PACK_CONTENTS_ICONS_SVG: &[u8] = include_bytes!("./archive/icons.svg");
pub(super) const PACK_CONTENTS_PALETTES: &[u8] = include_bytes!("./archive/palettes.json");
pub(super) const PACK_CONTENTS_SYNONYMS: &[u8] = include_bytes!("./archive/synonyms.json");
