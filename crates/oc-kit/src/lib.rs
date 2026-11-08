//! Shared building blocks for overcuriousity apps: design tokens, assets, security, identity and UI components.

pub mod assets;
pub mod config;
pub mod error;
pub mod tokens;

pub use error::OcError;
pub mod auth;
pub mod db;
pub mod identity;
pub mod prefs;
pub mod security;
pub mod session;
pub mod sse;
pub mod trash;
pub mod ui;
