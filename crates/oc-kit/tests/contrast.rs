use oc_kit::tokens::{contrast, parse_theme_blocks};

const TOKENS: &str = include_str!("../assets/css/00-tokens.css");
const BACKGROUNDS: [&str; 3] = ["bg-base", "bg-surface", "bg-elevated"];

fn check(
    theme: &std::collections::BTreeMap<String, String>,
    name: &str,
    fg: &str,
    bg: &str,
    floor: f64,
    failures: &mut Vec<String>,
) {
    let a = &theme[&format!("--color-{fg}")];
    let b = &theme[&format!("--color-{bg}")];
    let ratio = contrast(a, b);
    if ratio < floor {
        failures.push(format!("{name}: {fg} on {bg} is {ratio:.2}, needs {floor}"));
    }
}

#[test]
fn every_documented_pair_meets_its_floor_in_both_themes() {
    let (light, dark) = parse_theme_blocks(TOKENS);
    let mut failures = Vec::new();
    for (name, theme) in [("light", &light), ("dark", &dark)] {
        for bg in BACKGROUNDS {
            check(theme, name, "fg-primary", bg, 7.0, &mut failures);
            for fg in [
                "fg-secondary",
                "fg-muted",
                "accent",
                "danger",
                "warning",
                "success",
            ] {
                check(theme, name, fg, bg, 4.5, &mut failures);
            }
            check(theme, name, "border-control", bg, 3.0, &mut failures);
        }
        check(theme, name, "accent-fg", "accent", 4.5, &mut failures);
        for bg in ["bg-hover", "bg-active"] {
            check(theme, name, "fg-secondary", bg, 4.5, &mut failures);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn no_background_or_body_text_is_pure_black_or_white() {
    let (light, dark) = parse_theme_blocks(TOKENS);
    for theme in [&light, &dark] {
        for token in [
            "--color-bg-base",
            "--color-bg-surface",
            "--color-fg-primary",
        ] {
            assert!(
                !["#000000", "#ffffff"].contains(&theme[token].as_str()),
                "{token} is {}",
                theme[token]
            );
        }
    }
}

#[test]
fn the_dark_block_is_identical_under_media_query_and_data_theme() {
    let blocks = oc_kit::tokens::dark_blocks(TOKENS);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0], blocks[1]);
}
