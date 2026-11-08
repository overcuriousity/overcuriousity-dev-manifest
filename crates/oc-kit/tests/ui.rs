#![allow(clippy::unwrap_used)]

use oc_kit::auth::Branding;
use oc_kit::prefs::{Density, Prefs, PrefsSource, Theme};
use oc_kit::ui::*;

const B: Branding = Branding {
    app_name: "Test",
    logo_svg: "<svg></svg>",
};

#[test]
fn a_button_is_a_button_and_a_link_is_an_anchor() {
    assert!(
        Html::from(Button::primary("Save"))
            .0
            .starts_with("<button type=\"button\" class=\"oc-btn oc-btn--primary\"")
    );
    assert!(
        Html::from(Link::data("/x", "x"))
            .0
            .starts_with("<a href=\"/x\" class=\"oc-link oc-link--data\"")
    );
}

#[test]
fn an_icon_button_tooltip_names_action_and_shortcut() {
    let h = Html::from(IconButton::new(Icon::FilterTo, "Filter to this").shortcut("F")).0;
    assert!(
        h.contains("aria-label=\"Filter to this\"")
            && h.contains("data-oc-tip=\"Filter to this (F)\""),
        "{h}"
    );
}

#[test]
fn notices_carry_a_symbol_for_every_kind() {
    for (n, icon) in [
        (Notice::info("a"), "info"),
        (Notice::success("a"), "circle-check"),
        (Notice::warning("a"), "triangle-alert"),
        (Notice::danger("a"), "circle-x"),
    ] {
        assert!(Html::from(n).0.contains(&format!("data-icon=\"{icon}\"")));
    }
}

#[test]
fn components_nest_without_double_escaping_and_user_text_is_escaped() {
    let e = Html::from(Empty::new("<script>x</script>").action(Button::primary("Add")));
    assert!(
        e.0.contains("&#60;script&#62;") || e.0.contains("&lt;script&gt;"),
        "{}",
        e.0
    );
    assert!(e.0.contains("<button"), "{}", e.0);
    let f = Html::from(Facts {
        rows: vec![Fact::new("ID", "a\"b").copy()],
    })
    .0;
    assert!(!f.contains("a\"b") && f.contains("data-oc-copy="));
}

fn page(prefs: Prefs, src: PrefsSource) -> String {
    let top = TopBar::new(B, "/", prefs, None);
    Html::render(&Page::new(
        "T",
        Profile::Workbench,
        top,
        Html("<p>x</p>".into()),
        src,
    ))
    .0
}

#[test]
fn the_page_has_no_inline_script_or_style_and_loads_theme_js_in_head() {
    let p = page(Prefs::default(), PrefsSource::Local);
    let head = &p[..p.find("</head>").unwrap()];
    assert!(head.contains("js/theme.js\"></script>"));
    assert!(!p.contains(" style="));
    for (i, _) in p.match_indices("<script") {
        assert!(p[i..i + 80].contains(" src="), "inline script at {i}");
    }
    assert!(p.contains(r#"<meta name="htmx-config" content='{"allowEval":false"#));
}

#[test]
fn the_page_renders_server_preferences_on_html() {
    let p = page(
        Prefs {
            theme: Theme::Dark,
            density: Density::Compact,
        },
        PrefsSource::User,
    );
    assert!(
        p.contains(
            r#"<html lang="en" data-theme="dark" data-density="compact" data-prefs="user">"#
        ),
        "{}",
        &p[..200]
    );
    assert!(
        page(Prefs::default(), PrefsSource::Local)
            .contains(r#"<html lang="en" data-prefs="local">"#)
    );
}

#[test]
fn typed_confirm_renders_a_disabled_danger_button() {
    let t = TypedConfirm {
        id: "purge".into(),
        title: "Purge case".into(),
        text: "Gone for good.".into(),
        name_to_type: "ACME breach".into(),
        action_label: "Purge case".into(),
        post: HxAttrs::post("/cases/1/purge"),
    };
    let h = Html::from(t).0;
    assert!(
        h.contains("data-oc-confirm-name=\"ACME breach\"")
            && h.contains("class=\"oc-btn oc-btn--danger\" disabled")
    );
}

#[test]
fn every_icon_renders_with_aria_hidden_and_the_oc_icon_class() {
    for i in Icon::ALL {
        let s = i.sized(IconSize::Row).0;
        assert!(
            s.starts_with("<svg class=\"oc-icon oc-icon--row\"")
                && s.contains("aria-hidden=\"true\""),
            "{s}"
        );
    }
}
