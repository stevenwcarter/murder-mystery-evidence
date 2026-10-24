use axum::http::StatusCode;
use axum_test::{TestRequest, TestServer};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use murder_mystery_evidence::evidence::{EvidenceFiles, Round};
use murder_mystery_evidence::{AppState, router};

const PASSWORD: &str = "pw";

fn server(round: u8) -> TestServer {
    let round = Round::new(round).expect("valid test round");
    TestServer::new(router(AppState::with_round(PASSWORD, round)))
}

fn basic(password: &str) -> String {
    format!("Basic {}", STANDARD.encode(format!("detective:{password}")))
}

fn admin(req: TestRequest) -> TestRequest {
    req.add_header("authorization", basic(PASSWORD))
}

#[tokio::test]
async fn healthz_is_ok() {
    let server = TestServer::new(router(AppState::new(PASSWORD)));
    server.get("/healthz").await.assert_status_ok();
}

#[tokio::test]
async fn locked_evidence_is_404_and_identical_to_unknown() {
    let server = server(1);
    let locked = server.get("/evidence/round2-notebook.jpg").await;
    let unknown = server.get("/evidence/nope.jpg").await;
    locked.assert_status_not_found();
    unknown.assert_status_not_found();
    assert_eq!(locked.text(), unknown.text());
}

#[tokio::test]
async fn revealed_evidence_is_served() {
    let server = server(2);
    let res = server.get("/evidence/round2-notebook.jpg").await;
    res.assert_status_ok();
    res.assert_header("content-type", "image/jpeg");
    let expected = EvidenceFiles::get("Round 2 - Notebook.jpg").unwrap();
    assert_eq!(res.as_bytes().len(), expected.data.len());
    server
        .get("/evidence/round3-note.jpg")
        .await
        .assert_status_not_found();
}

#[tokio::test]
async fn admin_requires_basic_auth() {
    let server = server(1);
    let res = server.get("/admin").await;
    res.assert_status(StatusCode::UNAUTHORIZED);
    let challenge = res.header("www-authenticate");
    assert!(challenge.to_str().unwrap().starts_with("Basic"));
    server
        .get("/admin")
        .add_header("authorization", basic("wrong"))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
    admin(server.get("/admin")).await.assert_status_ok();
}

#[tokio::test]
async fn admin_sets_round() {
    let server = server(1);
    let res = admin(server.post("/admin/round"))
        .form(&[("round", "3")])
        .await;
    res.assert_status(StatusCode::SEE_OTHER);
    res.assert_header("location", "/admin");
    server
        .get("/evidence/round3-note.jpg")
        .await
        .assert_status_ok();
}

#[tokio::test]
async fn unauthenticated_post_does_not_change_round() {
    let server = server(1);
    server
        .post("/admin/round")
        .form(&[("round", "4")])
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
    server
        .get("/evidence/round4-view-1.jpg")
        .await
        .assert_status_not_found();
}

#[tokio::test]
async fn invalid_round_is_rejected() {
    let server = server(1);
    for value in ["0", "5", "abc"] {
        let res = admin(server.post("/admin/round"))
            .form(&[("round", value)])
            .await;
        assert!(
            matches!(
                res.status_code(),
                StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
            ),
            "round={value} gave {}",
            res.status_code()
        );
    }
    server
        .get("/evidence/round2-notebook.jpg")
        .await
        .assert_status_not_found();
}
