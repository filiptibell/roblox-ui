use anyhow::{bail, Context, Result};

/**
    An 8-bit RGB color.
*/
pub type Rgb = (u8, u8, u8);

pub fn color_from_hex(hex: impl AsRef<str>) -> Result<Rgb> {
    match hex.as_ref().trim().trim_start_matches('#') {
        s if s.len() == 6 => {
            let r = u8::from_str_radix(&s[..2], 16).context("invalid hexadecimal string")?;
            let g = u8::from_str_radix(&s[2..4], 16).context("invalid hexadecimal string")?;
            let b = u8::from_str_radix(&s[4..6], 16).context("invalid hexadecimal string")?;
            Ok((r, g, b))
        }
        s => bail!("color hex string must be 6 characters, got {}", s.len()),
    }
}

pub fn colors_are_similar(a: Rgb, b: Rgb) -> bool {
    let diffr = a.0.abs_diff(b.0) as u16;
    let diffg = a.1.abs_diff(b.1) as u16;
    let diffb = a.2.abs_diff(b.2) as u16;
    (diffr + diffg + diffb) <= 6
}
