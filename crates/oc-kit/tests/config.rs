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
    assert_eq!(e.to_string(), "public_url must be https unless it is localhost or allow_plain_http is set.");
}

#[test]
fn allow_plain_http_accepts_an_http_lan_url_outside_a_secure_context() {
    let d = cfg("public_url = \"http://notes.lan:8080\"\nallow_plain_http = true").deployment().unwrap();
    assert!(!d.secure && !d.secure_context);
    assert_eq!(d.origin, "http://notes.lan:8080");
    assert!(d.host_allowed("notes.lan:8080") && !d.host_allowed("notes.lan"));
}

#[test]
fn http_localhost_and_https_are_secure_contexts() {
    assert!(cfg("").deployment().unwrap().secure_context);
    assert!(cfg("public_url = \"https://notes.example\"").deployment().unwrap().secure_context);
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

#[test]
fn any_key_can_be_read_from_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("url");
    std::fs::write(&f, "https://notes.example\n").unwrap();
    temp_env::with_vars([("OCFILE__PUBLIC_URL_FILE", Some(f.to_str().unwrap()))], || {
        let c = load::<OcConfig>("ocfile", None).unwrap();
        assert_eq!(c.public_url.unwrap().as_str(), "https://notes.example/");
    });
}
