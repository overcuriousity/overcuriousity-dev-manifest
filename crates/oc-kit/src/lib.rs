//! Shared building blocks for overcuriousity apps: design tokens, assets, security, identity and UI components.

pub mod assets;
pub mod config;
pub mod error;
pub mod tokens;

pub use error::OcError;
pub mod db;
pub mod identity;
pub mod security;
