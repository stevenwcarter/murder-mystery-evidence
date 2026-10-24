//! HTTP Basic authentication for the admin routes.

use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use subtle::ConstantTimeEq;

use crate::state::AppState;

/// Middleware that admits requests whose Basic password matches the admin
/// password; the user name is ignored.
pub async fn require_admin(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let authorised = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(basic_password)
        .is_some_and(|password| matches_password(&password, &state.admin_password));
    if authorised {
        next.run(req).await
    } else {
        unauthorized()
    }
}

/// Extracts the password from an `Authorization: Basic` header value.
fn basic_password(value: &HeaderValue) -> Option<Vec<u8>> {
    let encoded = value.to_str().ok()?.strip_prefix("Basic ")?;
    let decoded = STANDARD.decode(encoded.trim()).ok()?;
    let colon = decoded.iter().position(|&b| b == b':')?;
    Some(decoded[colon + 1..].to_vec())
}

/// Constant-time comparison; differing lengths simply fail.
fn matches_password(given: &[u8], expected: &str) -> bool {
    given.len() == expected.len() && bool::from(given.ct_eq(expected.as_bytes()))
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(WWW_AUTHENTICATE, r#"Basic realm="detective""#)],
        "unauthorized",
    )
        .into_response()
}
