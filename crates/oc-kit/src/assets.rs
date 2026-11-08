//! Embedded static files served under a content-hashed path, so they can be cached forever.

use std::borrow::Cow;
use std::sync::OnceLock;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use rust_embed::RustEmbed;
use sha2::{Digest, Sha256};

#[derive(RustEmbed)]
#[folder = "assets/"]
#[exclude = "*.test.mjs"]
#[exclude = "css/[0-9][0-9]-*.css"]
struct KitFiles;

const KIT_CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/oc.css"));
const KIT_CSS_PATH: &str = "css/oc.css";

type Getter = fn(&str) -> Option<Cow<'static, [u8]>>;

pub struct AssetSet {
    mount: &'static str,
    hash: String,
    get: Getter,
}

fn kit_get(path: &str) -> Option<Cow<'static, [u8]>> {
    if path == KIT_CSS_PATH {
        return Some(Cow::Borrowed(KIT_CSS.as_bytes()));
    }
    KitFiles::get(path).map(|f| f.data)
}

fn hash_of(paths: impl Iterator<Item = Cow<'static, str>>, get: Getter) -> String {
    let mut paths: Vec<_> = paths.collect();
    paths.sort();
    let mut h = Sha256::new();
    for p in paths {
        h.update(p.as_bytes());
        if let Some(data) = get(&p) {
            h.update(&data);
        }
    }
    hex::encode(h.finalize())[..12].to_owned()
}

impl AssetSet {
    pub fn kit() -> &'static AssetSet {
        static KIT: OnceLock<AssetSet> = OnceLock::new();
        KIT.get_or_init(|| AssetSet {
            mount: "_oc",
            hash: hash_of(KitFiles::iter().chain([Cow::Borrowed(KIT_CSS_PATH)]), kit_get),
            get: kit_get,
        })
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn get(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        (self.get)(path)
    }

    pub fn url(&self, path: &str) -> String {
        debug_assert!(self.get(path).is_some(), "unknown asset {path}");
        format!("/{}/{}/{}", self.mount, self.hash, path)
    }

    fn router(&'static self) -> Router {
        Router::new().route(&format!("/{}/{{hash}}/{{*path}}", self.mount), get(serve)).with_state(self)
    }
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("css") => "text/css; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("woff2") => "font/woff2",
        Some("webmanifest") => "application/manifest+json",
        Some("png") => "image/png",
        Some("txt") => "text/plain; charset=utf-8",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

async fn serve(State(set): State<&'static AssetSet>, Path((hash, path)): Path<(String, String)>) -> Response {
    if hash != set.hash {
        return StatusCode::NOT_FOUND.into_response();
    }
    match set.get(&path) {
        Some(data) => (
            [
                (header::CONTENT_TYPE, content_type(&path)),
                (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            data.into_owned(),
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub fn asset_router() -> Router {
    AssetSet::kit().router()
}

/// Gives an app the same hashed, immutable serving for its own embedded files.
pub fn app_assets<E: RustEmbed>(mount: &'static str) -> (&'static AssetSet, Router) {
    let set: &'static AssetSet = Box::leak(Box::new(AssetSet {
        mount,
        hash: hash_of(E::iter(), |p| E::get(p).map(|f| f.data)),
        get: |p| E::get(p).map(|f| f.data),
    }));
    (set, set.router())
}
