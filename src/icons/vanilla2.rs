#![allow(dead_code)]

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use bytes::Bytes;
use const_format::concatcp;
use serde::Deserialize;
use tokio::join;

use resvg::{
    tiny_skia::Pixmap,
    usvg::{
        Options as SvgOptions, Transform as SvgTransform, Tree as SvgParserTree, TreeParsing as _,
    },
    Tree as SvgTree,
};

use super::*;

pub struct Vanilla2IconPack;

const ICON_RENDER_SCALE: u32 = 4;

const URL_REPO: &str = "Elttob/Vanilla";
const URL_COMMIT: &str = "7dd06bde94384b249055922f9818a87b3c3eba89";
const URL_BASE: &str = concatcp!(
    "https://raw.githubusercontent.com/",
    URL_REPO,
    "/",
    URL_COMMIT
);

const PACK_PALETTES_URL: &str = concatcp!(URL_BASE, "/docs/icons/palettes.json");
const PACK_ICON_DATA_URL: &str = concatcp!(URL_BASE, "/docs/icons/icondata.json");
const PACK_ICONS_SVG_URL: &str = concatcp!(URL_BASE, "/docs/icons/icons.svg");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaletteId {
    Platinum,
    Graphite,
    White,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaletteColor {
    Red,
    Yellow,
    Green,
    Blue,
    Purple,
    Grey,
}

#[derive(Debug, Clone, Deserialize)]
struct Palettes {
    defaults: HashMap<String, PaletteId>,
    palettes: Vec<Palette>,
}

#[derive(Debug, Clone, Deserialize)]
struct Palette {
    id: PaletteId,
    title: String,
    #[serde(alias = "page_colours")]
    page_colors: PaletteId,
    #[serde(alias = "colours")]
    colors: HashMap<PaletteColor, String>,
}

#[derive(Debug, Clone, Deserialize)]
struct IconData {
    icon: usize,
    name: String,
    glyph: String,
    #[serde(alias = "colour")]
    color: PaletteColor,
}

const PALETTE_ID_SOURCE: PaletteId = PaletteId::Platinum;

#[async_trait::async_trait]
impl IconPackProvider for Vanilla2IconPack {
    async fn download(&self) -> Result<IconPackContents> {
        let client = reqwest::Client::new();

        let (response_palettes, response_icondata, response_icons_svg) = join!(
            client.get(PACK_PALETTES_URL).send(),
            client.get(PACK_ICON_DATA_URL).send(),
            client.get(PACK_ICONS_SVG_URL).send()
        );

        let (bytes_palettes, bytes_icondata, bytes_icons_svg) = join!(
            response_palettes
                .context("failed to download palettes (1)")?
                .bytes(),
            response_icondata
                .context("failed to download icondata (1)")?
                .bytes(),
            response_icons_svg
                .context("failed to download icons svg (1)")?
                .bytes()
        );

        let bytes_palettes = bytes_palettes.context("failed to download palettes (2)")?;
        let bytes_icondata = bytes_icondata.context("failed to download palettes (2)")?;
        let bytes_icons_svg = bytes_icons_svg.context("failed to download icons svg (2)")?;

        std::fs::write("icons.svg", &bytes_icons_svg).ok();

        let palettes: Palettes = serde_json::from_slice(bytes_palettes.as_ref())
            .context("failed to deserialize palettes")?;
        let icon_datas: Vec<IconData> = serde_json::from_slice(bytes_icondata.as_ref())
            .context("failed to deserialize icondata")?;

        let palette_id_light = palettes
            .defaults
            .get("light")
            .context("missing default palette for 'light'")?;
        let palette_id_dark = palettes
            .defaults
            .get("dark")
            .context("missing default palette for 'dark'")?;

        let palette_source = palettes
            .palettes
            .iter()
            .find(|p| p.id == PALETTE_ID_SOURCE)
            .with_context(|| format!("failed to find source palette ({PALETTE_ID_SOURCE:?})"))?;
        let palette_light = palettes
            .palettes
            .iter()
            .find(|p| p.id == *palette_id_light)
            .context("failed to find light palette")?;
        let palette_dark = palettes
            .palettes
            .iter()
            .find(|p| p.id == *palette_id_dark)
            .context("failed to find dark palette")?;

        let mut contents = IconPackContents::new();
        for (path, bytes) in
            generate_palette_pngs(palette_source, palette_light, &icon_datas, &bytes_icons_svg)?
        {
            contents.insert_icon_light(path, bytes);
        }
        for (path, bytes) in
            generate_palette_pngs(palette_source, palette_dark, &icon_datas, &bytes_icons_svg)?
        {
            contents.insert_icon_dark(path, bytes);
        }

        Ok(contents)
    }
}

fn generate_palette_pngs(
    source_palette: &Palette,
    target_palette: &Palette,
    icon_datas: &[IconData],
    svg_bytes: &[u8],
) -> Result<Vec<(PathBuf, Bytes)>> {
    let mut svg_string = String::from_utf8(svg_bytes.to_vec())
        .context("failed to parse svg contents into utf8 string")?;

    // HACK: This hex color is not correct in the source data
    svg_string = svg_string.replace("006FB3", "006FB2");

    for (source_color, source_hex) in &source_palette.colors {
        let target_hex = target_palette
            .colors
            .get(source_color)
            .context("missing color in target palette")?;
        svg_string = svg_string.replace(source_hex, target_hex);
    }

    let options = SvgOptions::default();
    let svg_utree = SvgParserTree::from_data(svg_string.as_bytes(), &options)
        .context("failed to parse svg contents into tree")?;
    let svg_tree = SvgTree::from_usvg(&svg_utree);

    // FUTURE: When we have support for svg in the explorer,
    // we can skip rendering here and instead write svgs with
    // elements outside of view removed to keep svg size down
    let mut icons = Vec::new();
    for icon_data in icon_datas {
        if icon_data.name.contains("(unused)")
            || icon_data.name.contains("(alternative)")
            || icon_data.name.contains("(alt)")
        {
            continue;
        }

        let transform =
            SvgTransform::from_scale(ICON_RENDER_SCALE as f32, ICON_RENDER_SCALE as f32)
                .pre_translate(-16.0 * (icon_data.icon as f32), 0.0);

        let mut pixmap = Pixmap::new(16 * ICON_RENDER_SCALE, 16 * ICON_RENDER_SCALE).unwrap();
        svg_tree.render(transform, &mut pixmap.as_mut());
        let png = pixmap.encode_png().context("failed to encode icon png")?;

        icons.push((
            PathBuf::from(format!("{}.png", icon_data.name)),
            Bytes::from(png),
        ));
    }

    Ok(icons)
}
