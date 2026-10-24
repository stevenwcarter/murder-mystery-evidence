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

/// [`round_values`] as SSE `round` events.
pub fn round_stream(rx: watch::Receiver<Round>) -> impl Stream<Item = Result<Event, Infallible>> {
    round_values(rx).map(|round| Ok(round_event(round)))
}

/// `GET /events`: streams the round to viewers so they update live.
pub async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(round_stream(state.subscribe()))
        .keep_alive(KeepAlive::new().interval(KEEP_ALIVE_INTERVAL))
}

#[cfg(test)]
mod tests {
    use tokio::time::timeout;

    use super::*;

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
