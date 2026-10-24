//! Handlers that serve embedded files.

use std::path::Path as FsPath;

use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

use crate::evidence::{EvidenceFiles, find, visible};
use crate::state::AppState;

/// The stylesheet and script under `assets/`, embedded into the binary.
#[derive(Embed)]
#[folder = "assets/"]
pub struct StaticAssets;

/// Serves an evidence image once its round has been reached.
///
/// Locked and unknown slugs get the same response so locked files cannot be
/// told apart from files that do not exist.
pub async fn evidence_file(State(state): State<AppState>, Path(slug): Path<String>) -> Response {
    let file = find(&slug)
        .filter(|item| visible(item, state.current()))
        .and_then(|item| EvidenceFiles::get(item.file));
    match file {
        Some(file) => (
            [
                (CONTENT_TYPE, "image/jpeg"),
                (CACHE_CONTROL, "private, max-age=3600"),
            ],
            file.data.into_owned(),
        )
            .into_response(),
        None => not_found(),
    }
}

/// Serves a file from `assets/`.
///
/// `no-cache` plus a content-hash ETag makes phones revalidate every load,
/// so a redeploy during the party never leaves anyone on stale CSS or JS,
/// while unchanged files cost only a `304`.
pub async fn static_asset(Path(file): Path<String>, headers: HeaderMap) -> Response {
    let Some((asset, content_type)) = StaticAssets::get(&file).zip(content_type(&file)) else {
        return not_found();
    };
    let etag = format!(
        "\"{}\"",
        hex::encode(&asset.metadata.sha256_hash()[..ETAG_BYTES])
    );
    let not_modified = headers.get(IF_NONE_MATCH).is_some_and(|tag| tag == &etag);
    let tag = [(ETAG, etag)];
    let no_cache = (CACHE_CONTROL, "no-cache");
    if not_modified {
        return (StatusCode::NOT_MODIFIED, tag, [no_cache]).into_response();
    }
    let headers = [no_cache, (CONTENT_TYPE, content_type)];
    (tag, headers, asset.data.into_owned()).into_response()
}

/// Bytes of the SHA-256 kept in the ETag (16 hex digits).
const ETAG_BYTES: usize = 8;

/// The content type for an asset, or `None` for kinds the site never serves.
fn content_type(file: &str) -> Option<&'static str> {
    match FsPath::new(file).extension()?.to_str()? {
        "css" => Some("text/css; charset=utf-8"),
        "js" => Some("text/javascript; charset=utf-8"),
        _ => None,
    }
}

/// The single 404 response used for every missing or locked file.
pub fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "not found").into_response()
}
