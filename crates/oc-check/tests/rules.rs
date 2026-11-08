use std::path::Path;

use oc_check::{check_file, check_html};

fn rules(path: &str, text: &str) -> Vec<&'static str> {
    check_file(Path::new(path), text)
        .into_iter()
        .map(|f| f.rule)
        .collect()
}

#[test]
fn each_bad_snippet_triggers_exactly_its_rule() {
    let unsafe_inline = concat!("script-src 'unsafe", "-inline'");
    let cases: [(&str, String, &str); 9] = [
        ("a/x.css", ".a { color: #ff0000; }".into(), "literal-colour"),
        (
            "a/x.js",
            "el.textContent = 'rgb(1, 2, 3)';".into(),
            "literal-colour",
        ),
        (
            "a/x.html",
            "<p style=\"margin:0\">x</p>".into(),
            "inline-style",
        ),
        (
            "a/x.html",
            "<script>alert1()</script>".into(),
            "inline-script",
        ),
        ("a/x.html", "<button hx-on:click=\"x\">".into(), "hx-on"),
        (
            "a/x.js",
            "if (confirm(\"Sure?\")) go();".into(),
            "native-dialog",
        ),
        (
            "a/x.css",
            "@import url(https://cdn.example/x.css);".into(),
            "external-url",
        ),
        (
            "a/x.rs",
            format!("const CSP: &str = \"{unsafe_inline}\";"),
            "csp-unsafe",
        ),
        ("a/x.css", ".a { font-size: 0.8rem; }".into(), "small-text"),
    ];
    for (path, text, rule) in cases {
        assert_eq!(rules(path, &text), vec![rule], "{path}: {text}");
    }
}

#[test]
fn clean_snippets_pass() {
    assert!(rules("a/00-tokens.css", ":root { --c: #ffffff; }").is_empty());
    assert!(rules("a/x.css", ".a { color: var(--c); font-size: max(0.75rem, 12px); background: color-mix(in srgb, var(--c) 10%, transparent); }").is_empty());
    assert!(rules("a/x.html", "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg><a href=\"#main\">x</a><script src=\"/a.js\"></script>").is_empty());
    assert!(
        rules(
            "a/x.js",
            "export function confirmEnabled(a, b) { return a === b; }"
        )
        .is_empty()
    );
}

#[test]
fn vendor_files_may_contain_urls_but_not_unsafe_csp() {
    assert!(rules("assets/vendor/htmx.min.js", "var u = 'https://htmx.org';").is_empty());
    let bad = concat!("'unsafe", "-eval'");
    assert_eq!(rules("assets/vendor/x.js", bad), vec!["csp-unsafe"]);
}

#[test]
fn rendered_html_allows_the_theme_color_meta_only() {
    let ok =
        "<meta name=\"theme-color\" media=\"(prefers-color-scheme: light)\" content=\"#f8f6f1\">";
    assert!(check_html(ok).is_empty());
    assert_eq!(check_html("<div style=\"color:red\">").len(), 1);
}
