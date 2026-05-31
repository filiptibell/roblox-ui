use anyhow::{bail, Context, Result};

use usvg::Color;

pub fn color_from_hex(hex: impl AsRef<str>) -> Result<Color> {
    match hex.as_ref().trim().trim_start_matches('#') {
        s if s.len() == 6 => {
            let r = u8::from_str_radix(&s[..2], 16).context("invalid hexadecimal string")?;
            let g = u8::from_str_radix(&s[2..4], 16).context("invalid hexadecimal string")?;
            let b = u8::from_str_radix(&s[4..6], 16).context("invalid hexadecimal string")?;
            Ok(Color::new_rgb(r, g, b))
        }
        s => bail!("color hex string must be 6 characters, got {}", s.len()),
    }
}

pub fn colors_are_similar(a: &Color, b: &Color) -> bool {
    let diffr = a.red.abs_diff(b.red) as u16;
    let diffg = a.green.abs_diff(b.green) as u16;
    let diffb = a.blue.abs_diff(b.blue) as u16;
    (diffr + diffg + diffb) <= 6
}
