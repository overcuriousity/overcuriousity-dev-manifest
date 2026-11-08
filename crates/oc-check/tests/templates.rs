use std::fs;
use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(Path::parent).unwrap_or(Path::new("."))
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn shared(text: &str) -> Vec<String> {
    text.lines().filter(|l| !l.contains("# repo-specific")).map(str::to_owned).collect()
}

/// The repo's own workflow may add jobs, but every template line must appear in it, in order.
fn is_subsequence(needle: &[String], hay: &[String]) -> Result<(), String> {
    let mut it = hay.iter();
    for line in needle {
        if !it.any(|h| h == line) {
            return Err(line.clone());
        }
    }
    Ok(())
}

#[test]
fn own_workflows_contain_the_templates_except_repo_specific_lines() {
    for wf in ["ci.yml", "release.yml"] {
        let template = shared(&read(&format!("templates/.github/workflows/{wf}")));
        let own = shared(&read(&format!(".github/workflows/{wf}")));
        if let Err(line) = is_subsequence(&template, &own) {
            panic!("{wf}: template line missing from the repo's workflow: {line:?}");
        }
    }
    assert_eq!(read("templates/.github/dependabot.yml"), read(".github/dependabot.yml"));
}

#[test]
fn every_action_is_pinned_to_a_full_commit_sha() {
    for f in [
        "templates/.github/workflows/ci.yml",
        "templates/.github/workflows/release.yml",
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
    ] {
        for line in
            read(f).lines().filter(|l| l.trim_start().starts_with("- uses:") || l.trim_start().starts_with("uses:"))
        {
            let r = line.split("uses:").nth(1).unwrap_or_default().trim();
            let sha = r.split('@').nth(1).unwrap_or_default().split_whitespace().next().unwrap_or_default();
            assert!(sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()), "{f}: {line}");
            assert!(line.contains("# v"), "{f}: missing version comment: {line}");
        }
    }
}

#[test]
fn the_nginx_template_adds_no_headers_and_streams_sse() {
    let conf = read("templates/deploy/nginx.conf");
    assert!(!conf.contains("add_header"));
    assert!(conf.contains("proxy_buffering off") && conf.contains("X-Forwarded-Proto"));
}
