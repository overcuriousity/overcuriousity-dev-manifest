#![allow(clippy::unwrap_used, clippy::expect_used)]

use oc_kit::config::{OcConfig, load};

fn cfg(toml: &str) -> OcConfig {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("c.toml");
    std::fs::write(&path, toml).unwrap();
    load::<OcConfig>("octest", Some(&path)).unwrap()
}

#[test]
fn no_public_url_on_loopback_means_http_localhost() {
    let d = cfg("").deployment().unwrap();
    assert_eq!(d.origin, "http://localhost:8080");
    assert_eq!(d.rp_id, "localhost");
    assert!(!d.secure);
    assert!(d.host_allowed("127.0.0.1:8080") && d.host_allowed("localhost:8080"));
    assert!(!d.host_allowed("evil.example"));
}

#[test]
fn no_public_url_on_a_non_loopback_bind_is_refused() {
    let e = cfg("bind = \"0.0.0.0:8080\"").deployment().unwrap_err();
    assert_eq!(e.to_string(), "Set public_url when binding to a non-loopback address.");
}

#[test]
fn an_http_public_url_on_a_non_loopback_host_is_refused() {
    let e = cfg("public_url = \"http://notes.lan\"").deployment().unwrap_err();
    assert_eq!(e.to_string(), "public_url must be https unless it is localhost.");
}

#[test]
fn an_https_public_url_sets_secure_and_rp_id_to_its_host() {
    let d = cfg("public_url = \"https://notes.example\"").deployment().unwrap();
    assert!(d.secure);
    assert_eq!(d.rp_id, "notes.example");
    assert_eq!(d.origin, "https://notes.example");
    assert!(d.host_allowed("notes.example") && !d.host_allowed("localhost:8080"));
}

#[test]
fn env_overrides_and_file_secrets_are_read() {
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("s");
    std::fs::write(&secret, "hunter2\n").unwrap();
    temp_env::with_vars(
        [
            ("OCENV__BIND", Some("127.0.0.1:9000")),
            ("OCENV__AUTH__OIDC__ISSUER", Some("https://idp.example")),
            ("OCENV__AUTH__OIDC__CLIENT_ID", Some("app")),
            ("OCENV__AUTH__OIDC__CLIENT_SECRET_FILE", Some(secret.to_str().unwrap())),
        ],
        || {
            let c = load::<OcConfig>("ocenv", None).unwrap();
            assert_eq!(c.bind.port(), 9000);
            let oidc = c.auth.oidc.unwrap();
            assert_eq!(oidc.client_secret.expose(), "hunter2");
            assert_eq!(format!("{:?}", oidc.client_secret), "Secret(***)");
        },
    );
}

#[test]
fn unknown_keys_are_rejected_with_the_key_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("c.toml");
    std::fs::write(&path, "bnid = \"x\"").unwrap();
    let e = load::<OcConfig>("octest", Some(&path)).unwrap_err();
    assert!(e.to_string().contains("bnid"), "{e}");
}
