use std::fs;
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(Path::parent).unwrap_or(Path::new("."))
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn backticked_paths(text: &str) -> Vec<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .filter(|s| {
            (s.starts_with("manifest/")
                || s.starts_with("crates/")
                || s.starts_with("templates/")
                || s.starts_with("docs/"))
                && !s.contains(' ')
                && !s.contains('*')
                && !s.contains('<')
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_path_named_in_agents_readme_and_retrofit_exists() {
    for doc in ["AGENTS.md", "README.md", "docs/retrofit.md"] {
        for p in backticked_paths(&read(doc)) {
            assert!(root().join(p.trim_end_matches('/')).exists(), "{doc} names {p}, which does not exist");
        }
    }
}

#[test]
fn token_values_quoted_in_10_visual_match_the_token_file() {
    let css = read("crates/oc-kit/assets/css/00-tokens.css");
    let (light, dark) = oc_kit_tokens(&css);
    let visual = read("manifest/10-visual.md");
    let mut checked = 0;
    for line in visual.lines().filter(|l| l.starts_with("| `--color-")) {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let name = cells[1].trim_matches('`');
        let (l, d) = (cells[2].trim_matches('`'), cells[3].trim_matches('`'));
        if l.starts_with('#') {
            assert_eq!(light.get(name).map(String::as_str), Some(l), "{name} light");
            assert_eq!(dark.get(name).map(String::as_str), Some(d), "{name} dark");
            checked += 1;
        }
    }
    assert!(checked >= 15, "only {checked} token rows found");
}

fn oc_kit_tokens(
    css: &str,
) -> (std::collections::BTreeMap<String, String>, std::collections::BTreeMap<String, String>) {
    let block = |sel: &str| {
        let start = css.find(sel).map(|i| i + sel.len()).unwrap_or(0);
        let open = css[start..].find('{').map(|i| start + i + 1).unwrap_or(0);
        let close = css[open..].find('}').map(|i| open + i).unwrap_or(open);
        css[open..close]
            .split(';')
            .filter_map(|d| d.split_once(':'))
            .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
            .filter(|(k, v)| k.starts_with("--") && v.starts_with('#'))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let light = block(":root");
    let mut dark = light.clone();
    dark.extend(block(":root[data-theme=\"dark\"]"));
    (light, dark)
}

#[test]
fn the_csp_in_60_security_is_the_one_oc_kit_sends() {
    let src = read("crates/oc-kit/src/security/headers.rs");
    let csp = src.split("pub const CSP: &str = \"").nth(1).and_then(|s| s.split('"').next()).unwrap_or_default();
    assert!(!csp.is_empty());
    assert!(read("manifest/60-security.md").contains(csp), "60-security.md does not quote the CSP oc-kit sends");
}

#[test]
fn rule_lines_start_with_must_should_never_or_may() {
    for entry in fs::read_dir(root().join("manifest")).unwrap_or_else(|e| panic!("{e}")) {
        let path = entry.map(|e| e.path()).unwrap_or_default();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_owned();
        if name.starts_with("99-") || name == "sources.md" {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        for line in text.lines().filter(|l| l.starts_with("- ")) {
            assert!(
                ["- MUST", "- SHOULD", "- NEVER", "- MAY"].iter().any(|p| line.starts_with(p)),
                "{name}: top-level list item is not a rule: {line}"
            );
        }
    }
}
