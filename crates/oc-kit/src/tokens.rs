//! Parsing of `00-tokens.css`, used by the contrast test and by `oc-check`.

use std::collections::BTreeMap;

pub type Theme = BTreeMap<String, String>;

fn declarations(block: &str) -> Theme {
    block
        .split(';')
        .filter_map(|d| {
            let (k, v) = d.split_once(':')?;
            let k = k.trim();
            let v = v.trim();
            (k.starts_with("--") && v.starts_with('#')).then(|| (k.to_owned(), normalise_hex(v)))
        })
        .collect()
}

fn normalise_hex(v: &str) -> String {
    let h = v.trim_start_matches('#').to_ascii_lowercase();
    if h.len() == 3 { format!("#{}", h.chars().flat_map(|c| [c, c]).collect::<String>()) } else { format!("#{h}") }
}

fn block_after<'a>(css: &'a str, selector: &str) -> Option<&'a str> {
    let start = css.find(selector)? + selector.len();
    let open = css[start..].find('{')? + start + 1;
    let close = css[open..].find('}')? + open;
    Some(&css[open..close])
}

/// Light tokens come from the bare `:root` block; dark tokens are light tokens overlaid with the dark block.
pub fn parse_theme_blocks(css: &str) -> (Theme, Theme) {
    let light = block_after(css, ":root").map(declarations).unwrap_or_default();
    let mut dark = light.clone();
    if let Some(b) = block_after(css, ":root[data-theme=\"dark\"]") {
        dark.extend(declarations(b));
    }
    (light, dark)
}

pub fn dark_blocks(css: &str) -> Vec<Theme> {
    [":root:not([data-theme=\"light\"])", ":root[data-theme=\"dark\"]"]
        .iter()
        .filter_map(|s| block_after(css, s).map(declarations))
        .collect()
}

fn luminance(hex: &str) -> f64 {
    let h = hex.trim_start_matches('#');
    let channel = |i: usize| {
        let c = f64::from(u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0)) / 255.0;
        if c <= 0.039_28 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * channel(0) + 0.7152 * channel(2) + 0.0722 * channel(4)
}

pub fn contrast(a: &str, b: &str) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}
