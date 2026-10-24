//! Server-sent events announcing round changes.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use tokio_stream::{Stream, StreamExt};

use crate::evidence::Round;
use crate::state::AppState;

const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

/// The current round followed by every change.
pub fn round_values(rx: watch::Receiver<Round>) -> impl Stream<Item = Round> {
    WatchStream::new(rx)
}

fn round_event(round: Round) -> Event {
    Event::default().event("round").data(round.to_string())
}

/// [`round_values`] as SSE `round` events, ending when the server shuts down.
///
/// Open streams would otherwise keep graceful shutdown waiting forever.
pub fn round_stream(state: &AppState) -> impl Stream<Item = Result<Event, Infallible>> + use<> {
    let shutdown = tokio_stream::once(state.shutdown_signal()).then(|signal| signal);
    round_values(state.subscribe())
        .map(Some)
        .merge(shutdown.map(|()| None))
        .map_while(|round| round)
        .map(|round| Ok(round_event(round)))
}

/// `GET /events`: streams the round to viewers so they update live.
pub async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(round_stream(&state)).keep_alive(KeepAlive::new().interval(KEEP_ALIVE_INTERVAL))
}

#[cfg(test)]
mod tests {
    use tokio::time::timeout;

    use super::*;

    #[tokio::test]
    async fn stream_ends_on_shutdown() {
        let state = AppState::with_round("pw", Round::new(2).unwrap());
        let mut events = Box::pin(round_stream(&state));
        let first = timeout(Duration::from_secs(1), events.next()).await;
        assert!(matches!(first, Ok(Some(Ok(_)))));

        state.shutdown();
        let end = timeout(Duration::from_secs(1), events.next()).await;
        assert!(matches!(end, Ok(None)));
    }

    #[tokio::test]
    async fn stream_sends_current_then_changes() {
        let state = AppState::with_round("pw", Round::new(2).unwrap());
        let mut values = Box::pin(round_values(state.subscribe()));
        let first = timeout(Duration::from_secs(1), values.next()).await;
        assert_eq!(first, Ok(Some(Round::new(2).unwrap())));

        state.set(Round::new(3).unwrap());
        let second = timeout(Duration::from_secs(1), values.next()).await;
        assert_eq!(second, Ok(Some(Round::new(3).unwrap())));
    }
}
