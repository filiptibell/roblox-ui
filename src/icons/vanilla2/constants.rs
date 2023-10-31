use const_format::concatcp;

use super::*;

pub(super) const URL_REPO: &str = "Elttob/Vanilla";
pub(super) const URL_COMMIT: &str = "7dd06bde94384b249055922f9818a87b3c3eba89";
pub(super) const URL_BASE: &str = concatcp!(
    "https://raw.githubusercontent.com/",
    URL_REPO,
    "/",
    URL_COMMIT
);

pub(super) const PACK_PALETTES_URL: &str = concatcp!(URL_BASE, "/docs/icons/palettes.json");
pub(super) const PACK_ICON_DATA_URL: &str = concatcp!(URL_BASE, "/docs/icons/icondata.json");
pub(super) const PACK_ICONS_SVG_URL: &str = concatcp!(URL_BASE, "/docs/icons/icons.svg");

pub(super) const PALETTE_ID_SOURCE: PaletteId = PaletteId::Platinum;
