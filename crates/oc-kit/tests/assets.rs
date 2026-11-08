#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use oc_kit::assets::{AssetSet, asset_router};
use tower::ServiceExt;

const VOCABULARY: [&str; 39] = [
    "copy",
    "check",
    "filter",
    "filter-x",
    "pin",
    "archive",
    "trash-2",
    "undo-2",
    "pencil",
    "share-2",
    "download",
    "search",
    "settings",
    "x",
    "plus",
    "history",
    "link",
    "tag",
    "eye",
    "eye-off",
    "sun",
    "moon",
    "monitor",
    "a-large-small",
    "info",
    "circle-check",
    "triangle-alert",
    "circle-x",
    "ellipsis",
    "chevron-right",
    "chevron-down",
    "notebook",
    "folder",
    "user",
    "log-out",
    "panel-left",
    "book-open",
    "key-round",
    "list",
];

#[test]
fn every_vocabulary_icon_is_embedded() {
    for name in VOCABULARY {
        assert!(
            AssetSet::kit().get(&format!("icons/{name}.svg")).is_some(),
            "{name}"
        );
    }
}

#[tokio::test]
async fn asset_urls_carry_the_build_hash_and_are_immutable() {
    let url = AssetSet::kit().url("fonts/atkinson-next-400.woff2");
    assert!(url.starts_with(&format!("/_oc/{}/", AssetSet::kit().hash())));
    let res = asset_router()
        .oneshot(Request::get(&url).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        res.headers()[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .contains("immutable")
    );
    assert_eq!(res.headers()[header::CONTENT_TYPE], "font/woff2");
}

#[tokio::test]
async fn a_stale_hash_is_not_found() {
    let res = asset_router()
        .oneshot(
            Request::get("/_oc/000000000000/css/oc.css")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[test]
fn the_concatenated_css_starts_with_the_tokens_layer() {
    let css = AssetSet::kit().get("css/oc.css").unwrap();
    let css = std::str::from_utf8(&css).unwrap();
    assert!(css.contains("--color-bg-base"));
    assert!(css.find("--color-bg-base") < css.find("@font-face").or(Some(usize::MAX)));
}
