#![allow(clippy::unwrap_used)]

use oc_kit::assets::AssetSet;

fn css(path: &str) -> String {
    String::from_utf8(AssetSet::kit().get(path).unwrap().into_owned()).unwrap()
}

fn all_css() -> String {
    [
        "css/oc.css",
        "css/profile-workbench.css",
        "css/profile-everyday.css",
    ]
    .map(css)
    .join("\n")
}

fn rule<'a>(css: &'a str, selector: &str) -> &'a str {
    let start = css
        .find(selector)
        .unwrap_or_else(|| panic!("no rule {selector}"));
    let open = start + css[start..].find('{').unwrap();
    &css[open..open + css[open..].find('}').unwrap()]
}

#[test]
fn root_font_size_is_a_percentage_and_compact_is_87_5() {
    let c = css("css/oc.css");
    assert!(rule(&c, "html {").contains("font-size: 100%"));
    assert!(rule(&c, "html[data-density=\"compact\"] {").contains("font-size: 87.5%"));
}

#[test]
fn no_font_size_falls_below_12px_at_compact() {
    let c = all_css();
    for decl in c.split("font-size:").skip(1) {
        let value = decl.split([';', '}']).next().unwrap().trim();
        if value.starts_with("max(")
            || value.starts_with("var(")
            || value.ends_with('%')
            || value == "inherit"
        {
            continue;
        }
        let px = if let Some(rem) = value.strip_suffix("rem") {
            rem.parse::<f64>().unwrap() * 14.0
        } else if let Some(px) = value.strip_suffix("px") {
            px.parse::<f64>().unwrap()
        } else {
            panic!("unexpected font-size {value}");
        };
        assert!(px >= 12.0, "font-size {value} renders at {px}px in compact");
    }
}

#[test]
fn reduced_motion_disables_transitions_and_animations() {
    let c = css("css/oc.css");
    let block = &c[c.find("prefers-reduced-motion: reduce").unwrap()..];
    assert!(block[..400].contains("transition: none") && block[..400].contains("animation: none"));
}

#[test]
fn selection_uses_a_frame_not_a_left_bar() {
    let c = css("css/oc.css");
    let r = rule(&c, ".oc-table tr.is-selected {");
    assert!(r.contains("outline: 1px solid var(--color-accent)"));
    assert!(!r.contains("border-left") && !r.contains("inset"));
}

#[test]
fn muted_text_switches_to_secondary_on_hovered_and_selected_rows() {
    let c = css("css/oc.css");
    assert!(rule(&c, ".oc-table tr:hover .oc-muted").contains("var(--color-fg-secondary)"));
}

#[test]
fn form_fields_use_the_control_border() {
    let c = css("css/oc.css");
    assert!(rule(&c, ".oc-input {").contains("var(--color-border-control)"));
}

#[test]
fn only_the_float_shadow_is_used() {
    for decl in all_css().split("box-shadow:").skip(1) {
        let value = decl.split([';', '}']).next().unwrap().trim();
        assert!(
            value == "var(--shadow-float)" || value == "none",
            "box-shadow {value}"
        );
    }
}
