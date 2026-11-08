use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let roots: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let roots = if roots.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        roots
    };
    let findings: Vec<_> = roots.iter().flat_map(|r| oc_check::check_tree(r)).collect();
    for (path, f) in &findings {
        println!("{}:{}: {}: {}", path.display(), f.line, f.rule, f.message);
    }
    if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("oc-check: {} finding(s)", findings.len());
        ExitCode::FAILURE
    }
}
