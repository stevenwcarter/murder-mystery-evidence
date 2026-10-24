//! Handlers that serve embedded files.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};

use crate::evidence::{EvidenceFiles, find, visible};
use crate::state::AppState;

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

/// The single 404 response used for every missing or locked file.
pub fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "not found").into_response()
}
