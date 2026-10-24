use axum_test::TestServer;
use murder_mystery_evidence::{AppState, router};

#[tokio::test]
async fn healthz_is_ok() {
    let server = TestServer::new(router(AppState::new("pw")));
    server.get("/healthz").await.assert_status_ok();
}
