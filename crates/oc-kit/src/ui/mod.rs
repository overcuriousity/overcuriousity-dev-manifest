//! Server-rendered UI pieces. Every component renders through an Askama template in `templates/oc/`
//! and implements `HtmlSafe`, so app templates embed it with `{{ component }}` without re-escaping.

mod components;
mod icons;
mod shell;

use std::fmt;

pub use components::*;
pub use icons::{Icon, IconSize};
pub use shell::*;

/// Already-escaped HTML.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Html(pub String);

impl Html {
    pub fn render<T: askama::Template>(t: &T) -> Html {
        Html(t.render().unwrap_or_else(|e| {
            tracing::error!(error = %e, "template render failed");
            String::new()
        }))
    }

    pub fn join(parts: impl IntoIterator<Item = Html>) -> Html {
        Html(parts.into_iter().map(|h| h.0).collect())
    }
}

impl fmt::Display for Html {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl askama::filters::HtmlSafe for Html {}

impl axum::response::IntoResponse for Html {
    fn into_response(self) -> axum::response::Response {
        axum::response::Html(self.0).into_response()
    }
}

macro_rules! html_safe {
    ($($t:ty),* $(,)?) => {$(
        impl askama::filters::HtmlSafe for $t {}
        impl From<$t> for Html {
            fn from(t: $t) -> Html { Html::render(&t) }
        }
    )*};
}
pub(crate) use html_safe;

/// Attributes for htmx and data hooks. Only `hx-*` (never `hx-on`), `data-*` and `aria-*` names are allowed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HxAttrs(pub Vec<(&'static str, String)>);

impl HxAttrs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, name: &'static str, value: impl Into<String>) -> Self {
        debug_assert!(
            (name.starts_with("hx-") && !name.starts_with("hx-on"))
                || name.starts_with("data-")
                || name.starts_with("aria-"),
            "attribute {name} is not allowed"
        );
        self.0.push((name, value.into()));
        self
    }

    pub fn get(url: impl Into<String>) -> Self {
        Self::new().with("hx-get", url)
    }

    pub fn post(url: impl Into<String>) -> Self {
        Self::new().with("hx-post", url)
    }

    pub fn target(self, sel: impl Into<String>) -> Self {
        self.with("hx-target", sel)
    }

    pub fn swap(self, s: impl Into<String>) -> Self {
        self.with("hx-swap", s)
    }

    pub fn vals(self, json: impl Into<String>) -> Self {
        self.with("hx-vals", json)
    }
}

impl fmt::Display for HxAttrs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (k, v) in &self.0 {
            write!(f, " {k}=\"{}\"", askama::filters::escape(v, askama::filters::Html).map_err(|_| fmt::Error)?)?;
        }
        Ok(())
    }
}

impl askama::filters::HtmlSafe for HxAttrs {}
