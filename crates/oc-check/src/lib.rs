//! The mechanical half of `manifest/99-review-checklist.md`.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub line: usize,
    pub rule: &'static str,
    pub message: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Css,
    Html,
    Js,
    Rust,
}

fn re(p: &str) -> Regex {
    Regex::new(p).unwrap_or_else(|e| panic!("bad rule pattern {p}: {e}"))
}

static HEX: LazyLock<Regex> = LazyLock::new(|| re(r"#[0-9a-fA-F]{3,8}\b"));
static COLOR_FN: LazyLock<Regex> = LazyLock::new(|| re(r"\b(rgba?|hsla?)\("));
static STYLE_ATTR: LazyLock<Regex> = LazyLock::new(|| re(r#"\sstyle\s*="#));
static SCRIPT_TAG: LazyLock<Regex> = LazyLock::new(|| re(r"<script\b[^>]*>"));
static HX_ON: LazyLock<Regex> = LazyLock::new(|| re(r"\bhx-on"));
static NATIVE_DIALOG: LazyLock<Regex> = LazyLock::new(|| re(r"\b(confirm|alert|prompt)\s*\("));
static URL: LazyLock<Regex> = LazyLock::new(|| re(r"https?://[^\s\x22')]+"));
static CSP_UNSAFE: LazyLock<Regex> =
    LazyLock::new(|| re(concat!("'unsafe", r"-(inline|eval|hashes)'")));
static FONT_SIZE: LazyLock<Regex> = LazyLock::new(|| re(r"font-size:\s*([0-9.]+)(px|rem)\b"));

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const COMPACT_ROOT_PX: f64 = 14.0;

fn kind(path: &Path) -> Option<Kind> {
    match path.extension()?.to_str()? {
        "css" => Some(Kind::Css),
        "html" => Some(Kind::Html),
        "js" | "mjs" => Some(Kind::Js),
        "rs" => Some(Kind::Rust),
        _ => None,
    }
}

fn check(text: &str, kind: Kind, is_tokens: bool, is_vendor: bool, rendered: bool) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut push = |line: usize, rule: &'static str, message: String| {
        out.push(Finding {
            line,
            rule,
            message,
        })
    };
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if let Some(m) = CSP_UNSAFE.find(line) {
            push(
                n,
                "csp-unsafe",
                format!("{} weakens the CSP; the manifest forbids it.", m.as_str()),
            );
        }
        if kind == Kind::Rust {
            continue;
        }
        let theme_meta = rendered && line.contains("name=\"theme-color\"");
        if !is_tokens
            && !is_vendor
            && !theme_meta
            && let Some(m) = HEX.find(line).or_else(|| COLOR_FN.find(line))
        {
            push(
                n,
                "literal-colour",
                format!(
                    "{} is a literal colour; use a token from 00-tokens.css.",
                    m.as_str()
                ),
            );
        }
        if kind == Kind::Html {
            if STYLE_ATTR.is_match(line) {
                push(
                    n,
                    "inline-style",
                    "style= attributes break the strict CSP; use a class.".into(),
                );
            }
            for tag in SCRIPT_TAG.find_iter(line) {
                if !tag.as_str().contains("src=") {
                    push(
                        n,
                        "inline-script",
                        "Inline scripts break the strict CSP; load a same-origin file.".into(),
                    );
                }
            }
            if HX_ON.is_match(line) {
                push(
                    n,
                    "hx-on",
                    "hx-on needs eval; use a JS module instead.".into(),
                );
            }
        }
        if kind != Kind::Css && !is_vendor && NATIVE_DIALOG.is_match(line) {
            push(
                n,
                "native-dialog",
                "Native confirm/alert/prompt is banned; use undo or the typed-name dialog.".into(),
            );
        }
        if !is_vendor {
            for m in URL
                .find_iter(line)
                .filter(|m| !m.as_str().starts_with(SVG_NS))
            {
                push(
                    n,
                    "external-url",
                    format!(
                        "{} points outside the app; vendor the file instead.",
                        m.as_str()
                    ),
                );
            }
        }
        if kind == Kind::Css {
            for c in FONT_SIZE.captures_iter(line) {
                let value: f64 = c[1].parse().unwrap_or(0.0);
                let px = if &c[2] == "rem" {
                    value * COMPACT_ROOT_PX
                } else {
                    value
                };
                if px < 12.0 {
                    push(
                        n,
                        "small-text",
                        format!(
                            "font-size {}{} renders at {px:.1}px in compact; the floor is 12px.",
                            &c[1], &c[2]
                        ),
                    );
                }
            }
        }
    }
    out
}

pub fn check_file(path: &Path, text: &str) -> Vec<Finding> {
    let Some(kind) = kind(path) else {
        return Vec::new();
    };
    let is_tokens = path.file_name().is_some_and(|n| n == "00-tokens.css");
    let is_vendor = path.components().any(|c| c.as_os_str() == "vendor");
    check(text, kind, is_tokens, is_vendor, false)
}

/// Rules for a page the app rendered, used by apps' tests against every route.
pub fn check_html(text: &str) -> Vec<Finding> {
    check(text, Kind::Html, false, false, true)
}

const SKIP_DIRS: [&str; 6] = [
    "target",
    ".git",
    ".superpowers",
    ".claude",
    "node_modules",
    ".sqlx",
];

pub fn check_tree(root: &Path) -> Vec<(std::path::PathBuf, Finding)> {
    let mut out = Vec::new();
    let walker = walkdir::WalkDir::new(root).into_iter().filter_entry(|e| {
        !(e.file_type().is_dir()
            && e.file_name()
                .to_str()
                .is_some_and(|n| SKIP_DIRS.contains(&n)))
    });
    for entry in walker
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if path.to_str().is_some_and(|p| p.ends_with(".test.mjs")) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        out.extend(
            check_file(path, &text)
                .into_iter()
                .map(|f| (path.to_path_buf(), f)),
        );
    }
    out
}
