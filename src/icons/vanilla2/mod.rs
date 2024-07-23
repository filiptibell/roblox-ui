use std::path::PathBuf;

use anyhow::{Context, Result};
use bytes::Bytes;
use tokio::join;

use usvg::{
    NodeExt as _, NodeKind, NonZeroRect, Options as SvgOptions, Paint, Rect, Size, Tree as SvgTree,
    TreeParsing as _, TreeWriting as _, ViewBox, XmlOptions,
};

use super::*;

mod constants;
mod structs;
mod svg;

use constants::*;
use structs::*;
use svg::*;

pub struct Vanilla2;

impl IconPackProvider for Vanilla2 {
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
            generate_svgs(palette_source, palette_light, &icon_datas, &bytes_icons_svg)?
        {
            contents.insert_icon_light(path, bytes);
        }
        for (path, bytes) in
            generate_svgs(palette_source, palette_dark, &icon_datas, &bytes_icons_svg)?
        {
            contents.insert_icon_dark(path, bytes);
        }

        Ok(contents)
    }
}

fn generate_svgs(
    source_palette: &Palette,
    target_palette: &Palette,
    icon_datas: &[IconData],
    svg_bytes: &[u8],
) -> Result<Vec<(PathBuf, Bytes)>> {
    let svg_options = SvgOptions::default();
    let xml_options = XmlOptions::default();

    let svg_tree = SvgTree::from_data(svg_bytes, &svg_options)
        .context("failed to parse svg contents into tree")?;

    let mut icons = Vec::new();
    for icon_data in icon_datas {
        // The icon pack contains some files that we can safely ignore,
        // and they conveniently have some special naming we can match on
        if icon_data.name.contains("(unused)")
            || icon_data.name.contains("(alternative)")
            || icon_data.name.contains("(alt)")
        {
            continue;
        }

        // NOTE: Cloning the node here only clones the reference,
        // but we want a completely new node tree to manipulate
        let mut icon_tree = svg_tree.clone();
        icon_tree.root = svg_tree.root.make_deep_copy();
        icon_tree.size = Size::from_wh(16.0, 16.0).unwrap();
        icon_tree.view_box = ViewBox {
            aspect: icon_tree.view_box.aspect,
            rect: NonZeroRect::from_xywh(16.0 * (icon_data.icon as f32), 0.0, 16.0, 16.0).unwrap(),
        };

        optimize_svg_for_viewbox(&mut icon_tree)?;
        apply_palette_to_tree(source_palette, target_palette, &mut icon_tree)?;

        icons.push((
            PathBuf::from(format!("{}.svg", icon_data.name)),
            Bytes::from(icon_tree.to_string(&xml_options)),
        ));
    }

    Ok(icons)
}

fn optimize_svg_for_viewbox(svg_tree: &mut SvgTree) -> Result<()> {
    let view_box = Rect::from_ltrb(
        svg_tree.view_box.rect.left(),
        svg_tree.view_box.rect.top(),
        svg_tree.view_box.rect.right(),
        svg_tree.view_box.rect.bottom(),
    )
    .context("view box has invalid size")?;

    for child in svg_tree.root.children() {
        if let Some(bbox) = child.calculate_bbox() {
            if view_box.intersect(&bbox).is_none() {
                child.detach();
            }
        }
    }

    Ok(())
}

fn apply_palette_to_tree(
    source_palette: &Palette,
    target_palette: &Palette,
    svg_tree: &mut SvgTree,
) -> Result<()> {
    if source_palette == target_palette {
        return Ok(());
    }

    for descendant in svg_tree.root.descendants() {
        if let NodeKind::Path(path) = &mut *descendant.borrow_mut() {
            if let Some(fill) = path.fill.as_mut() {
                let mut new_fill = fill.clone();
                new_fill.paint =
                    apply_palette_to_paint(source_palette, target_palette, &fill.paint)?;
                path.fill = Some(new_fill);
            }
        }
    }

    Ok(())
}

fn apply_palette_to_paint(
    source_palette: &Palette,
    target_palette: &Palette,
    paint: &Paint,
) -> Result<Paint> {
    Ok(match paint {
        Paint::LinearGradient(_) => paint.clone(),
        Paint::RadialGradient(_) => paint.clone(),
        Paint::Pattern(_) => paint.clone(),
        Paint::Color(color) => {
            let source_key = source_palette
                .colors
                .iter()
                .find_map(|(source_key, source_hex)| {
                    if color_from_hex(source_hex)
                        .map(|c| colors_are_similar(color, &c))
                        .unwrap_or_default()
                    {
                        Some(source_key)
                    } else {
                        None
                    }
                });
            if let Some(source_key) = source_key {
                let target_hex = target_palette
                    .colors
                    .get(source_key)
                    .context("missing color in target palette")?;
                Paint::Color(color_from_hex(target_hex)?)
            } else {
                Paint::Color(*color)
            }
        }
    })
}
