#![allow(clippy::unwrap_used)]

use std::process::{Command, Output};

fn run(dir: &std::path::Path, toml: &str) -> Output {
    let file = dir.join("config.toml");
    std::fs::write(&file, toml).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_demo"));
    for (k, _) in std::env::vars() {
        if k.starts_with("DEMO__") {
            cmd.env_remove(k);
        }
    }
    cmd.env("DEMO__DATA_DIR", dir.join("data")).arg("--config").arg(&file).arg("check-config").output().unwrap()
}

#[test]
fn check_config_ok() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(dir.path(), "public_url = \"https://demo.example.org\"\nbind = \"127.0.0.1:9123\"\n");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "bind=127.0.0.1:9123\npublic_url=https://demo.example.org\n");
    assert!(!dir.path().join("data").exists());
}

#[test]
fn check_config_bad() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(dir.path(), "bind = \"0.0.0.0:9123\"\n");
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("public_url"));
}
