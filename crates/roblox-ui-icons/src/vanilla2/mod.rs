use std::fmt::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use bytes::Bytes;
use usvg::{tiny_skia_path::PathSegment, Node, Options, Paint, Transform, Tree};

use super::*;

mod constants;
mod structs;
mod svg;

use constants::*;
use structs::*;
use svg::*;

pub struct Vanilla2;

impl IconPackProvider for Vanilla2 {
    async fn get(&self) -> Result<IconPackContents> {
        let palettes: Palettes = serde_json::from_slice(PACK_CONTENTS_PALETTES)
            .context("failed to deserialize palettes")?;
        let icon_datas: Vec<IconData> = serde_json::from_slice(PACK_CONTENTS_ICON_DATA)
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

        // Parse the icon sprite sheet once. usvg 0.44 trees are immutable, so
        // rather than mutating it we read each path's absolute geometry + fill
        // out of it and re-render one small SVG per icon below.
        let tree = Tree::from_data(PACK_CONTENTS_ICONS_SVG, &Options::default())
            .context("failed to parse icons svg")?;
        let sprites = collect_sprite_paths(&tree);

        let mut contents = IconPackContents::new();
        for (path, bytes) in generate_svgs(palette_source, palette_light, &icon_datas, &sprites) {
            contents.insert_icon_light(path, bytes);
        }
        for (path, bytes) in generate_svgs(palette_source, palette_dark, &icon_datas, &sprites) {
            contents.insert_icon_dark(path, bytes);
        }

        Ok(contents)
    }
}

/**
    A single path extracted from the sprite sheet, in absolute coordinates.
*/
struct SpritePath {
    /// Left edge of the path's absolute bounding box.
    left: f32,
    /// Right edge of the path's absolute bounding box.
    right: f32,
    /// The SVG path `d` attribute, in absolute sprite-sheet coordinates.
    data: String,
    /// The path's solid fill color, if any.
    fill: Option<Rgb>,
    /// The fill opacity in the range 0.0..=1.0.
    opacity: f32,
    /// Whether the fill uses the even-odd rule.
    even_odd: bool,
}

/**
    Walks the parsed tree and extracts every path with its absolute geometry.
*/
fn collect_sprite_paths(tree: &Tree) -> Vec<SpritePath> {
    let mut out = Vec::new();
    collect_from_group(tree.root(), 1.0, &mut out);
    out
}

/**
    `inherited_opacity` is the product of all ancestor group opacities, which
    usvg keeps on the group rather than baking into each path's fill.
*/
fn collect_from_group(group: &usvg::Group, inherited_opacity: f32, out: &mut Vec<SpritePath>) {
    let group_opacity = inherited_opacity * group.opacity().get();
    for node in group.children() {
        match node {
            Node::Group(child) => collect_from_group(child, group_opacity, out),
            Node::Path(path) => {
                let bbox = path.abs_bounding_box();

                let fill = path.fill().and_then(|fill| match fill.paint() {
                    Paint::Color(color) => Some((color.red, color.green, color.blue)),
                    _ => None,
                });
                let even_odd = path
                    .fill()
                    .map(|f| matches!(f.rule(), usvg::FillRule::EvenOdd))
                    .unwrap_or(false);
                let fill_opacity = path.fill().map(|f| f.opacity().get()).unwrap_or(1.0);
                let opacity = group_opacity * fill_opacity;

                out.push(SpritePath {
                    left: bbox.x(),
                    right: bbox.x() + bbox.width(),
                    data: path_to_abs_d(path),
                    fill,
                    opacity,
                    even_odd,
                });
            }
            _ => {}
        }
    }
}

/**
    Renders a usvg path's geometry to an SVG `d` string, applying the path's
    absolute transform so the result is in sprite-sheet coordinates.
*/
fn path_to_abs_d(path: &usvg::Path) -> String {
    let ts = path.abs_transform();
    let mut d = String::new();
    for segment in path.data().segments() {
        match segment {
            PathSegment::MoveTo(p) => {
                let (x, y) = map(ts, p.x, p.y);
                let _ = write!(d, "M{} {}", num(x), num(y));
            }
            PathSegment::LineTo(p) => {
                let (x, y) = map(ts, p.x, p.y);
                let _ = write!(d, "L{} {}", num(x), num(y));
            }
            PathSegment::QuadTo(p1, p) => {
                let (x1, y1) = map(ts, p1.x, p1.y);
                let (x, y) = map(ts, p.x, p.y);
                let _ = write!(d, "Q{} {} {} {}", num(x1), num(y1), num(x), num(y));
            }
            PathSegment::CubicTo(p1, p2, p) => {
                let (x1, y1) = map(ts, p1.x, p1.y);
                let (x2, y2) = map(ts, p2.x, p2.y);
                let (x, y) = map(ts, p.x, p.y);
                let _ = write!(
                    d,
                    "C{} {} {} {} {} {}",
                    num(x1),
                    num(y1),
                    num(x2),
                    num(y2),
                    num(x),
                    num(y)
                );
            }
            PathSegment::Close => d.push('Z'),
        }
    }
    d
}

fn map(ts: Transform, x: f32, y: f32) -> (f32, f32) {
    (ts.sx * x + ts.kx * y + ts.tx, ts.ky * x + ts.sy * y + ts.ty)
}

/**
    Formats a float compactly (no trailing zeros).
*/
fn num(value: f32) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    let s = format!("{rounded}");
    if s == "-0" {
        "0".to_string()
    } else {
        s
    }
}

fn generate_svgs(
    source_palette: &Palette,
    target_palette: &Palette,
    icon_datas: &[IconData],
    sprites: &[SpritePath],
) -> Vec<(PathBuf, Bytes)> {
    // When the source and target palettes are the same (e.g. the light pack,
    // whose default palette IS the source palette) we leave the sprite-sheet
    // colors exactly as-is, matching the original generator.
    let recolor_pairs = if source_palette == target_palette {
        Vec::new()
    } else {
        build_recolor_pairs(source_palette, target_palette)
    };

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

        let offset = icon_data.icon * 16;
        let mut svg = format!(
            "<svg width=\"16\" height=\"16\" viewBox=\"{offset} 0 16 16\" \
             fill=\"none\" xmlns=\"http://www.w3.org/2000/svg\">"
        );

        // Include any path whose bounding box intersects this icon's 16px
        // window, matching the original generator (a path straddling the column
        // boundary belongs to both neighbours). Content fully outside the
        // window is excluded; anything partly outside is cropped by the viewBox.
        let lo = offset as f32;
        let hi = lo + 16.0;
        for sprite in sprites.iter().filter(|s| s.right >= lo && s.left <= hi) {
            svg.push_str("<path d=\"");
            svg.push_str(&sprite.data);
            svg.push('"');
            if let Some(color) = sprite.fill {
                let (r, g, b) = recolor(color, &recolor_pairs);
                let _ = write!(svg, " fill=\"#{r:02X}{g:02X}{b:02X}\"");
            }
            if sprite.opacity < 0.999 {
                let _ = write!(svg, " fill-opacity=\"{}\"", num(sprite.opacity));
            }
            if sprite.even_odd {
                svg.push_str(" fill-rule=\"evenodd\" clip-rule=\"evenodd\"");
            }
            svg.push_str("/>");
        }

        svg.push_str("</svg>");

        icons.push((
            PathBuf::from(format!("{}.svg", icon_data.name)),
            Bytes::from(svg),
        ));
    }

    icons
}

/**
    Builds `(source_rgb, target_rgb)` pairs for every named palette color.
*/
fn build_recolor_pairs(source: &Palette, target: &Palette) -> Vec<(Rgb, Rgb)> {
    source
        .colors
        .iter()
        .filter_map(|(key, source_hex)| {
            let target_hex = target.colors.get(key)?;
            let source_rgb = color_from_hex(source_hex).ok()?;
            let target_rgb = color_from_hex(target_hex).ok()?;
            Some((source_rgb, target_rgb))
        })
        .collect()
}

/**
    Maps a color to its target-palette equivalent, matching the source palette
    with a small tolerance (the sprite sheet colors are not always exact).
*/
fn recolor(color: Rgb, pairs: &[(Rgb, Rgb)]) -> Rgb {
    pairs
        .iter()
        .find(|(source, _)| colors_are_similar(color, *source))
        .map(|(_, target)| *target)
        .unwrap_or(color)
}
