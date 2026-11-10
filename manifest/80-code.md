# 80 · Code

- MUST: comment only where the code is not self-explanatory, and then say why, briefly.
- NEVER: narrate obvious steps, restate signatures in prose, write section-banner comments, or write a comment longer than the code it explains.
- MUST: one `thiserror` error type per crate with an HTTP status mapping; user-facing errors are one actionable sentence.
- NEVER: `unwrap()` or `expect()` outside tests.
- MUST: `tracing` for logs; no secrets in logs; authorization and cookie headers are redacted.
- MUST: configuration is a TOML file plus `APP__SECTION__KEY` environment overrides; secrets may come from files (`*_FILE` or `file://`); `config.example.toml` is committed; app settings live under `[app]`.
- MUST: an optional subsystem that is not configured shows no UI entry point.
- MUST: test names read as sentences (`a_deleted_note_can_be_restored_within_30_days`); tests use real SQLite, not mocks.
- MUST: keep the contrast test, the OpenAPI/route parity test and a test that checks rendered pages against the CSP rules (`oc_check::check_html_for` does this).
- MUST: rustfmt and clippy with the lint table from `templates/lints.toml`.
- NEVER: commit agent working files (`.superpowers/`, `.claude/`, generated specs, plans, scratch). `docs/` holds reference documentation only.
