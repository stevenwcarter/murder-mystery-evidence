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
    locked.assert_header("cache-control", "no-store");
    unknown.assert_header("cache-control", "no-store");
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
    let locked = || server.get("/evidence/round2-notebook.jpg");
    for value in ["0", "5", "abc", "300"] {
        let res = admin(server.post("/admin/round"))
            .form(&[("round", value)])
            .await;
        res.assert_status(StatusCode::BAD_REQUEST);
        locked().await.assert_status_not_found();
    }
    let res = admin(server.post("/admin/round"))
        .content_type("application/x-www-form-urlencoded")
        .await;
    res.assert_status(StatusCode::BAD_REQUEST);
    locked().await.assert_status_not_found();
}

#[tokio::test]
async fn guest_round1_shows_waiting_message() {
    let html = server(1).get("/").await.text();
    assert!(html.contains(
        "The investigation is underway. Evidence will appear here as the detective uncovers it."
    ));
    assert!(html.contains("<title>Jazz, Gin, Money &amp; Murder</title>"));
    assert!(html.contains("Round 1 of 4"));
    assert!(!html.contains("<figure"));
}

#[tokio::test]
async fn guest_round3_shows_rounds_2_and_3_only() {
    let res = server(3).get("/").await;
    res.assert_status_ok();
    res.assert_header("cache-control", "no-store");
    let html = res.text();
    for expected in [
        "The Notebook",
        "The Note",
        "round3-note.jpg",
        "Round 4 · sealed",
        r#"data-round="3""#,
    ] {
        assert!(html.contains(expected), "missing {expected}");
    }
    for locked in ["The Study", "round4-view-1.jpg", "round4-view-2.jpg"] {
        assert!(!html.contains(locked), "leaked {locked}");
    }
}

#[tokio::test]
async fn guest_rounds_are_newest_first() {
    let html = server(4).get("/").await.text();
    let newest = html.find(r#"id="round-4""#).expect("round 4 section");
    let oldest = html.find(r#"id="round-2""#).expect("round 2 section");
    assert!(newest < oldest);
}

#[tokio::test]
async fn admin_page_marks_current() {
    let res = admin(server(2).get("/admin")).await;
    res.assert_status_ok();
    let html = res.text();
    assert!(html.contains("Current round: 2"));
    assert_eq!(html.matches(r#"name="round""#).count(), 4);
    assert_eq!(html.matches(r#"class="current""#).count(), 1);
    let form = html
        .split("<form")
        .find(|form| form.contains(r#"value="2""#))
        .expect("form for round 2");
    assert!(form.contains(r#"class="current""#));
}

#[tokio::test]
async fn assets_have_etag_and_no_cache() {
    let server = server(1);
    let res = server.get("/assets/style.css").await;
    res.assert_status_ok();
    let content_type = res.header("content-type");
    assert!(content_type.to_str().unwrap().starts_with("text/css"));
    res.assert_header("cache-control", "no-cache");
    let etag = res.header("etag");
    server
        .get("/assets/style.css")
        .add_header("if-none-match", etag)
        .await
        .assert_status(StatusCode::NOT_MODIFIED);
    server
        .get("/assets/missing.css")
        .await
        .assert_status_not_found();
}
