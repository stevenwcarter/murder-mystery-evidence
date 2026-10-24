# Evidence Site Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A public, Art-Deco-themed site at `mm.carterfam.us` that shows murder-mystery evidence for the current round and earlier ones, advanced live by the host from a password-protected admin page.

**Architecture:** One Axum binary with server-rendered HTML (maud), images and static assets embedded with rust-embed, the current round in a `tokio::sync::watch` channel, SSE pushing round changes so guest pages reload themselves. Deployed as a single-replica Deployment on the homelab cluster from `~/k8s-config`.

**Tech Stack:** Rust edition 2024, axum 0.8, tokio, maud, rust-embed, clap, tracing, axum-test; musl + scratch Docker image; ArgoCD manifests in `~/k8s-config`.

**Spec:** `docs/superpowers/specs/2026-10-24-evidence-site-design.md` (read it first; this plan argues from it).

## Global Constraints

- Follow `~/.claude/rust-crate-decisions.md`: edition 2024 in `Cargo.toml` and `rustfmt.toml`; no `openssl`/`native-tls` anywhere (`cargo tree -i openssl` and `cargo tree -i native-tls` print nothing); `tikv-jemallocator` as global allocator under `#[cfg(target_env = "musl")]`; release profile `lto = true`, `codegen-units = 1`, `opt-level = 3`; `default-features = false` where sensible; commit `Cargo.lock`.
- Every task ends with `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` green.
- Rounds are `1..=4`; round 1 means nothing revealed. `MAX_ROUND = 4`.
- Env: `MM_LISTEN` (default `0.0.0.0:8080`), `MM_ADMIN_PASSWORD` (required), `MM_INITIAL_ROUND` (default `1`), `RUST_LOG` (default `info`).
- Image name `murder-mystery-evidence`; cluster pulls `192.168.42.134:5000/murder-mystery-evidence:<tag>`, desktop pushes `docker.wind-halfbeak.ts.net/murder-mystery-evidence:<tag>`.
- The app repo is `~/src/murder-mystery-evidence` (remote `origin` on GitHub, branch `main`); the deployment lives in `~/k8s-config` (`kubernetes/apps/murder-mystery/`). Commit in whichever repo a task touches; never force-push.
- Never print or commit the admin password; the plain value lives only in `~/k8s-config/_out/murder-mystery-admin-password`.

## Review Focus

1. A guest who opened the page at round 2 and whose phone slept for an hour: on wake the EventSource reconnects and the page must end up showing the current round (test: the SSE stream sends the current round immediately on every new subscription, Task 3).
2. `GET /evidence/<slug>` for a locked round must be byte-for-byte the same 404 as for an unknown slug (Task 3 test compares status and body).
3. `POST /admin/round` with `round=0`, `round=5`, `round=abc` or no body must not change the round and must return 400 (Task 3).
4. The `/` page must never contain a locked round's title or slug anywhere in its HTML, including the sealed cards (Task 4 test asserts absence of `round4-view-1.jpg` and "The Study" at round 3).
5. `/assets/*` after a redeploy: a changed `style.css` must get a different ETag and `Cache-Control: no-cache` so phones re-fetch (Task 4 test checks headers; ETag equality is checked for the same content served twice).

---

## File structure

- `Cargo.toml`, `Cargo.lock`, `rustfmt.toml`, `.gitignore`, `.dockerignore`, `Dockerfile`, `README.md`
- `src/main.rs` — clap `Cli` (`Serve` default, `Healthcheck`), tracing init, jemalloc, calls `server::run` / `healthcheck::run`
- `src/config.rs` — `Config::from_env() -> anyhow::Result<Config>`
- `src/healthcheck.rs` — TCP probe of the serving port
- `src/evidence.rs` — `Round`, `Item`, `EVIDENCE`, `visible`, `find`
- `src/state.rs` — `AppState` with the watch channel and password
- `src/auth.rs` — Basic-auth check for `/admin*`
- `src/assets.rs` — rust-embed structs and the `/evidence/{slug}` and `/assets/{file}` handlers
- `src/events.rs` — SSE stream from the watch receiver
- `src/pages.rs` — maud: layout, guest page, admin page
- `src/server.rs` — `router(state) -> Router`, handlers for `/`, `/admin`, `/admin/round`, `/healthz`, `run(config)`
- `assets/style.css`, `assets/app.js`
- `tests/http.rs` — axum-test integration tests (the router is a library item so tests can build it; expose `pub fn router` via `src/lib.rs`)

Use a `src/lib.rs` that declares the modules and re-exports `router`, `AppState`, `Config`; `main.rs` is thin.

---

### Task 1: Scaffold, config, CLI, healthcheck

**Files:**
- Modify: `Cargo.toml`
- Create: `rustfmt.toml`, `src/lib.rs`, `src/config.rs`, `src/healthcheck.rs`, `src/server.rs`, `src/state.rs` (minimal), `tests/http.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces: `pub struct Config { pub listen: SocketAddr, pub admin_password: String, pub initial_round: u8 }` with `pub fn from_env() -> anyhow::Result<Config>` and `pub fn from_vars(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Config>` (the env version calls the vars version with `std::env::var(..).ok()`).
- Produces: `pub fn router(state: AppState) -> axum::Router` in `server.rs` (only `/healthz` for now), `pub async fn run(config: Config) -> anyhow::Result<()>`.
- Produces: `pub fn run(listen: SocketAddr) -> std::process::ExitCode` in `healthcheck.rs`: connect TCP, write `GET /healthz HTTP/1.0\r\n\r\n`, read at least one byte within 3 s → 0, else 1.
- Produces: `AppState` placeholder: `#[derive(Clone)] pub struct AppState { pub admin_password: Arc<str> }` with `pub fn new(admin_password: &str) -> Self` (Task 3 adds the round channel; keep `new` so Task 3 extends rather than renames).

- [ ] **Step 1: Dependencies.** `cargo add axum@0.8 tokio --features tokio/full` plus `anyhow clap --features clap/derive`, `tracing`, `tracing-subscriber --features env-filter`, `tower-http --features compression-gzip`, and dev-deps `axum-test`. Add `[target.'cfg(target_env = "musl")'.dependencies] tikv-jemallocator = "0.6"`. Add the release profile from Global Constraints. Create `rustfmt.toml` with `edition = "2024"`.

- [ ] **Step 2: Write the failing tests** in `src/config.rs` (unit) and `tests/http.rs`:

```rust
// src/config.rs
#[test] fn missing_password_is_an_error() {
    assert!(Config::from_vars(|_| None).is_err());
}
#[test] fn defaults_apply() {
    let c = Config::from_vars(|k| (k == "MM_ADMIN_PASSWORD").then(|| "pw".into())).unwrap();
    assert_eq!(c.listen, "0.0.0.0:8080".parse().unwrap());
    assert_eq!(c.initial_round, 1);
}
#[test] fn initial_round_out_of_range_is_an_error() { /* MM_INITIAL_ROUND=5 → Err */ }
// tests/http.rs
#[tokio::test] async fn healthz_is_ok() {
    let server = axum_test::TestServer::new(murder_mystery_evidence::router(AppState::new("pw"))).unwrap();
    server.get("/healthz").await.assert_status_ok();
}
```

- [ ] **Step 3: Run `cargo test`**, expect compile failures (missing items).
- [ ] **Step 4: Implement** `config.rs`, `state.rs`, `server.rs` (`/healthz` → `"ok"`; `run` binds `config.listen`, logs the address at `info`, serves with graceful shutdown on ctrl-c/SIGTERM), `healthcheck.rs`, `main.rs` (clap: `#[command(version, about)]`, `enum Command { Serve, Healthcheck }`, no subcommand = `Serve`; `Healthcheck` reads `MM_LISTEN` for the port; tracing per the crate-decisions snippet; jemalloc global allocator under musl cfg).
- [ ] **Step 5: Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.** All green.
- [ ] **Step 6: Commit** `feat: scaffold server with config, CLI and healthcheck` (include `Cargo.lock`).

---

### Task 2: Evidence manifest and round gating

**Files:**
- Create: `src/evidence.rs`
- Modify: `src/lib.rs`
- Add dependency: `rust-embed` (features default).

**Interfaces:**
- Produces: `#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)] pub struct Round(u8)` with `pub const MIN: Round`, `pub const MAX: Round` (1 and 4), `pub fn new(n: u8) -> Option<Round>`, `pub fn get(self) -> u8`, `pub fn all() -> impl Iterator<Item = Round>`, `impl Display`.
- Produces: `pub struct Item { pub round: Round, pub slug: &'static str, pub title: &'static str, pub file: &'static str }` and `pub static EVIDENCE: [Item; 4]` with the spec's table (slugs `round2-notebook.jpg`, `round3-note.jpg`, `round4-view-1.jpg`, `round4-view-2.jpg`; files are the exact names in `Evidence/`).
- Produces: `pub fn visible(item: &Item, current: Round) -> bool` (`item.round <= current`), `pub fn find(slug: &str) -> Option<&'static Item>`, `pub fn for_round(round: Round) -> impl Iterator<Item = &'static Item>`.
- Produces: `#[derive(rust_embed::Embed)] #[folder = "Evidence/"] pub struct EvidenceFiles;`

- [ ] **Step 1: Write failing unit tests** in `evidence.rs`: `round_new_rejects_0_and_5`; `visible_is_round_le_current` (round-2 item visible at 2 and 4, not at 1); `find_known_and_unknown_slug`; `every_manifest_file_is_embedded` (`EvidenceFiles::get(item.file).is_some()` for all four); `slugs_are_unique`.
- [ ] **Step 2: Run `cargo test evidence`**, expect compile failure.
- [ ] **Step 3: Implement** the module.
- [ ] **Step 4: Run tests, clippy, fmt.** Green.
- [ ] **Step 5: Commit** `feat: evidence manifest, rounds and gating`.

---

### Task 3: State, admin auth, round changes, gated images, SSE

**Files:**
- Modify: `src/state.rs`, `src/server.rs`, `src/lib.rs`, `tests/http.rs`
- Create: `src/auth.rs`, `src/assets.rs` (evidence handler only), `src/events.rs`
- Add dependencies: `subtle`, `base64`, `tokio-stream --features sync`, `futures-util` (or `futures`), `axum` feature `form` is default.

**Interfaces:**
- Consumes: `Round`, `find`, `visible`, `EvidenceFiles` from Task 2.
- Produces in `state.rs`: `AppState { round: watch::Sender<Round> (wrapped in Arc), admin_password: Arc<str> }`, `pub fn new(admin_password: &str) -> Self` (round 1), `pub fn with_round(admin_password: &str, round: Round) -> Self`, `pub fn current(&self) -> Round`, `pub fn set(&self, round: Round)`, `pub fn subscribe(&self) -> watch::Receiver<Round>`.
- Produces in `auth.rs`: `pub async fn require_admin(State(state): State<AppState>, req: Request, next: Next) -> Response` middleware: parses `Authorization: Basic`, base64-decodes, splits on the first `:`, compares the password with `subtle::ConstantTimeEq`; failure → `401` with `WWW-Authenticate: Basic realm="detective"`.
- Produces in `assets.rs`: `pub async fn evidence_file(State(state): State<AppState>, Path(slug): Path<String>) -> Response`: `find(&slug)` filtered by `visible(item, state.current())` → 200 `image/jpeg`, `Cache-Control: private, max-age=3600`, body from `EvidenceFiles::get(item.file)`; otherwise `not_found()` = plain `404` with body `"not found"` (one function used for both cases).
- Produces in `events.rs`: `pub fn round_stream(rx: watch::Receiver<Round>) -> impl Stream<Item = Result<Event, Infallible>>` emitting `Event::default().event("round").data(n.to_string())` for the current value first, then every change (`tokio_stream::wrappers::WatchStream::new(rx)` does exactly this); `pub async fn events(State(state): State<AppState>) -> Sse<...>` with `KeepAlive::new().interval(Duration::from_secs(15))`.
- Produces in `server.rs`: `#[derive(Deserialize)] pub struct RoundForm { round: u8 }`; `POST /admin/round` → `Round::new(form.round)` → `state.set` + `303 See Other` to `/admin`, else `400`; routes `/evidence/{slug}`, `/events`; `/admin` and `/admin/round` under `route_layer(middleware::from_fn_with_state(state.clone(), require_admin))`. `GET /admin` returns a temporary `200` text `"round N"` (Task 4 replaces it). `run` builds the state from `Config` (`with_round`).

- [ ] **Step 1: Write failing tests** in `tests/http.rs` (helper `fn server(round: u8) -> TestServer` and `fn admin(req) -> req.add_header("authorization", "Basic " + base64("detective:pw"))`):
  - `locked_evidence_is_404_and_identical_to_unknown`: at round 1, `/evidence/round2-notebook.jpg` and `/evidence/nope.jpg` both 404 with equal bodies.
  - `revealed_evidence_is_served`: at round 2, `/evidence/round2-notebook.jpg` is 200 with `content-type: image/jpeg` and body length == `EvidenceFiles::get(..).len()`; `/evidence/round3-note.jpg` is 404.
  - `admin_requires_basic_auth`: `GET /admin` → 401 with `www-authenticate` header starting `Basic`; with a wrong password → 401; with the right one → 200.
  - `admin_sets_round`: authenticated `POST /admin/round` form `round=3` → 303, `location: /admin`; then `/evidence/round3-note.jpg` is 200.
  - `unauthenticated_post_does_not_change_round`: unauthenticated `POST round=4` → 401 and `/evidence/round4-view-1.jpg` is still 404.
  - `invalid_round_is_400`: authenticated `round=0`, `round=5`, `round=abc` → 400 (for `abc` axum's Form rejection is 422 or 400; accept either, assert the round did not change).
  - In `events.rs` unit test `stream_sends_current_then_changes`: `with_round(.., 2)`, take the stream, first item is `data: 2`-equivalent (compare the `Event` by formatting via `.into()`? simpler: have `round_stream` built on a tested helper `pub fn round_event(r: Round) -> Event` and test `round_values(rx)` = a `Stream<Item = Round>` that `round_stream` maps; assert first `Some(Round 2)`, after `state.set(3)` next is `Some(Round 3)`, each under `tokio::time::timeout(1s)`).
- [ ] **Step 2: Run `cargo test`**, expect failures.
- [ ] **Step 3: Implement** the modules and routes.
- [ ] **Step 4: Run tests, clippy, fmt.** Green.
- [ ] **Step 5: Commit** `feat: admin round control, gated evidence files and SSE`.

---

### Task 4: Pages, theme and client script

**Files:**
- Create: `src/pages.rs`, `assets/style.css`, `assets/app.js`
- Modify: `src/server.rs` (`/` and `GET /admin` use pages; `/assets/{file}` route), `src/assets.rs` (`StaticAssets` embed + `static_asset` handler), `tests/http.rs`
- Add dependency: `maud --features axum`, `sha2` (ETag), `hex` or `base16ct`.
- Before writing CSS/HTML, load the `frontend-design:frontend-design` skill (`Skill` tool) for the design pass; the theme tokens and motifs are in the spec's "Theme" section and the decoration photos are in `theme/`.

**Interfaces:**
- Consumes: `AppState`, `Round`, `EVIDENCE`, `for_round`, `visible`, `require_admin`.
- Produces in `pages.rs`: `pub fn guest(current: Round) -> maud::Markup`, `pub fn admin(current: Round) -> maud::Markup`. Guest page: `<body data-round="N">`; header title `Jazz, Gin, Money & Murder`, subtitle `Evidence Locker`, line `Round N of 4`; at round 1 the exact message `The investigation is underway. Evidence will appear here as the detective uncovers it.`; revealed rounds descending, `<section class="round" id="round-N">` with `<h2>Round N</h2>` and `<figure class="card">` per item containing `<a class="view" href="/evidence/<slug>" data-title="..."><img src="/evidence/<slug>" alt="<title>" loading="lazy"></a>`, `<figcaption>` title, `<a download href="/evidence/<slug>">Download</a>`; one `<section class="round sealed">` per locked round with text `Round N · sealed` and nothing else; a `<dialog id="viewer">` with `<img>`, close, download and "Open image" link; footer live indicator `<span id="live">`. Admin page: `Current round: N` and four `<form method="post" action="/admin/round">` each with `<input type="hidden" name="round" value="N">` and a `<button>` (`Round 1 · Nothing revealed`, `Round 2`, `Round 3`, `Round 4`), the current one `class="current"`.
- Produces in `assets.rs`: `#[derive(Embed)] #[folder = "assets/"] pub struct StaticAssets;` and `pub async fn static_asset(Path(file): Path<String>, headers: HeaderMap) -> Response`: content type from the extension (`.css` → `text/css; charset=utf-8`, `.js` → `text/javascript; charset=utf-8`), `ETag: "<hex sha256 prefix 16>"`, `Cache-Control: no-cache`; `If-None-Match` equal → 304; unknown file → the same `not_found()`.
- Produces in `server.rs`: `GET /` → `guest(state.current())` with `Cache-Control: no-store`; `GET /admin` → `admin(...)`; `GET /assets/{file}`; `tower_http::compression::CompressionLayer` on the router (SSE responses are excluded automatically for `text/event-stream`; verify by checking the `/events` response has no `content-encoding`).
- Produces `assets/app.js`: EventSource on `/events`, reload when `event.data !== document.body.dataset.round`; `#live` text/class on `open`/`error`; viewer: clicking `a.view` opens the dialog with that image (prevent default), pinch-zoom/pan with pointer events (transform `translate(x,y) scale(s)`, `s` clamped `1..6`, two-pointer scale around the midpoint, one-pointer pan when `s > 1`, double-tap toggles `1`/`2.5`), close on button, backdrop tap and Escape; download link and open-in-new-tab link point at the current image.
- Produces `assets/style.css`: the spec's tokens as CSS custom properties, Google Fonts `@import` for the chosen display face and Cormorant Garamond, double gold hairline frames with stepped corners on `.card` and `.round`, 1-column grid under 640px and 2 columns above, `.sealed` dim, `dialog#viewer` full-screen black with `touch-action: none` on the image stage, large thumb-friendly admin buttons (min 56px tall), `prefers-reduced-motion` respected.

- [ ] **Step 1: Write failing tests** in `tests/http.rs`:
  - `guest_round1_shows_waiting_message`: `/` at round 1 contains the exact message and no `<figure`.
  - `guest_round3_shows_rounds_2_and_3_only`: contains `The Notebook`, `The Note`, `round3-note.jpg`; does **not** contain `The Study`, `round4-view-1.jpg`, `round4-view-2.jpg`; contains `Round 4 · sealed`; `data-round="3"`; `cache-control: no-store`.
  - `guest_rounds_are_newest_first`: at round 4, index of `id="round-4"` < index of `id="round-2"`.
  - `admin_page_marks_current`: authenticated `/admin` at round 2 contains four `name="round"` inputs and `class="current"` exactly once, next to value `2`.
  - `assets_have_etag_and_no_cache`: `/assets/style.css` 200, `content-type` starts `text/css`, `cache-control: no-cache`, `etag` present; a second request with `if-none-match` set to it → 304; `/assets/missing.css` → 404.
  - `events_are_not_compressed`: `/events` with `accept-encoding: gzip`… skip if TestServer cannot read a partial stream; instead assert in a unit test that the compression layer is built with `.compress_when(|…| content_type != text/event-stream)` is unnecessary because tower-http excludes `text/event-stream` by default — add a one-line comment citing that in `server.rs` instead of a test.
- [ ] **Step 2: Run `cargo test`**, expect failures.
- [ ] **Step 3: Implement** `pages.rs`, the asset handler, the router changes, the CSS and JS. Then `cargo run` with `MM_ADMIN_PASSWORD=pw`, open `http://localhost:8080/` and `/admin` in a browser (use the Chrome tools if available, else `curl`) and check the page renders at 390px width without horizontal scroll, the viewer opens, and setting a round on `/admin` reloads the guest tab.
- [ ] **Step 4: Run tests, clippy, fmt.** Green.
- [ ] **Step 5: Commit** `feat: themed guest and admin pages with live reload and viewer`.

---

### Task 5: Docker image and app README

**Files:**
- Create: `Dockerfile`, `.dockerignore`, `README.md`

**Interfaces:**
- Consumes: the `healthcheck` subcommand (Task 1), `MM_*` env.

- [ ] **Step 1: Write `Dockerfile`** following `~/src/chore-tracker/Dockerfile` and the crate-decisions policy: stage 1 `docker.io/blackdex/rust-musl:x86_64-musl`, copy `Cargo.toml Cargo.lock`, stub `src/main.rs`, `cargo build --release`; stage 2 copy `src assets Evidence`, `touch src/main.rs`, `cargo build --release`; final `FROM scratch`, `COPY` the binary to `/murder-mystery-evidence`, `USER 65532:65532`, `ENV MM_LISTEN=0.0.0.0:8080`, `EXPOSE 8080`, `HEALTHCHECK --interval=30s --timeout=3s CMD ["/murder-mystery-evidence", "healthcheck"]`, `ENTRYPOINT ["/murder-mystery-evidence"]`. `.dockerignore`: `target`, `.git`, `theme`, `docs`.
- [ ] **Step 2: Build** `docker build -t murder-mystery-evidence:dev .` — expect success.
- [ ] **Step 3: Verify the spec's sequence**: `docker run -d --rm -p 18080:8080 -e MM_ADMIN_PASSWORD=pw --name mm murder-mystery-evidence:dev`; `curl -s localhost:18080/` contains the waiting message; `curl -s -o /dev/null -w '%{http_code}' localhost:18080/evidence/round2-notebook.jpg` is `404`; `curl -s -u detective:pw -d round=2 -o /dev/null -w '%{http_code}' localhost:18080/admin/round` is `303`; the evidence URL is now `200`; `curl -s -N --max-time 2 localhost:18080/events` prints `event: round` / `data: 2`; `docker inspect --format '{{.State.Health.Status}}' mm` becomes `healthy` within ~35 s; `docker stop mm`. Also confirm `cargo tree -i openssl` and `cargo tree -i native-tls` print nothing.
- [ ] **Step 4: Write `README.md`**: what it is, env vars, `cargo run`, the build/push commands (`docker build -t docker.wind-halfbeak.ts.net/murder-mystery-evidence:0.1.0 . && docker push docker.wind-halfbeak.ts.net/murder-mystery-evidence:0.1.0`), pointer to the spec and to `~/k8s-config/kubernetes/apps/murder-mystery/`.
- [ ] **Step 5: Commit** `build: musl/scratch Dockerfile and README`.

---

### Task 6: Cluster manifests and docs (in `~/k8s-config`)

**Files (all under `~/k8s-config`):**
- Create: `kubernetes/apps/murder-mystery/application.yaml`, `kubernetes/apps/murder-mystery/manifests/murder-mystery.yaml`, `.../httproute.yaml`, `.../network-policy.yaml`, `.../admin-sealed.yaml`
- Modify: `renovate.json` (own-images automerge list), `CLAUDE.md` (apps bullet list and the app-source table), `README.md` (new `## murder-mystery` section after `## candyland`), `TODO.md` (remove after the party)

**Interfaces:**
- Consumes: image `192.168.42.134:5000/murder-mystery-evidence:0.1.0`, port 8080, `/healthz`, env `MM_ADMIN_PASSWORD`.

- [ ] **Step 1: Generate the password** into `_out/murder-mystery-admin-password` without printing it: three lower-case words from `/usr/share/dict/words` if present (letters only, 4–8 chars, joined with `-`), else `openssl rand -base64 12`; `chmod 600`. Do not `cat` it. Seal: `kubernetes/bootstrap/seal.sh murder-mystery murder-mystery-admin kubernetes/apps/murder-mystery/manifests/admin-sealed.yaml --from-file=password=_out/murder-mystery-admin-password`.
- [ ] **Step 2: Write the manifests** per the spec's "Deployment" section, copying structure from `kubernetes/apps/candyland/` (Application, Deployment security context, Service, HTTPRoute with defaulted fields spelled out, CiliumNetworkPolicy). Differences: 1 replica, `strategy: {type: Recreate}`, `env MM_ADMIN_PASSWORD` via `secretKeyRef` (`murder-mystery-admin` / `password`), probes on `/healthz`, port name `http` 8080, hostname `mm.carterfam.us`. Header comments explain the one-replica rule (in-memory round + SSE).
- [ ] **Step 3: Validate**: `kubectl apply --dry-run=server -f kubernetes/apps/murder-mystery/manifests/` succeeds for every file except the namespace-dependent ones (create the namespace in the dry run order or use `--dry-run=client`), and `kubectl apply --dry-run=client -f kubernetes/apps/murder-mystery/application.yaml`.
- [ ] **Step 4: Docs**: `renovate.json` add `"192.168.42.134:5000/murder-mystery-evidence"` to the "Our own app releases" rule; `CLAUDE.md` apps list bullet (`murder-mystery` (public, `https://mm.carterfam.us`, one-night evidence site for the 2026-10-24 party; in-memory round state, one replica; admin at `/admin`, password in `_out/murder-mystery-admin-password`; remove after the party)) and app-source table (`~/src/murder-mystery-evidence`); `README.md` section with the live URL, how the host advances rounds, the restart caveat, the build/push line and teardown steps (delete the directory, then `kubectl delete ns murder-mystery`); `TODO.md` item dated 2026-10-25.
- [ ] **Step 5: Commit** in `~/k8s-config` (do not push yet): `murder-mystery: evidence site for the 2026-10-24 party`. Commit trailer per the harness.

---

### Task 7: Build, push, deploy, verify (orchestrator runs this)

- [ ] **Step 1:** In the app repo: `git push origin main`. `docker build -t docker.wind-halfbeak.ts.net/murder-mystery-evidence:0.1.0 . && docker push docker.wind-halfbeak.ts.net/murder-mystery-evidence:0.1.0`.
- [ ] **Step 2:** In `~/k8s-config`: `git fetch && git rebase origin/main` (Renovate may have pushed), `git push origin HEAD:main`, then `kubectl -n argocd annotate application root argocd.argoproj.io/refresh=normal --overwrite` and, once the child exists, the same on `murder-mystery`.
- [ ] **Step 3:** Verify: `kubectl -n argocd get application murder-mystery` Synced/Healthy; `kubectl -n murder-mystery get pods` Running 1/1; `curl -s -H 'Host: mm.carterfam.us' http://192.168.42.20/ | grep -c 'investigation is underway'` is 1; `dig +short mm.carterfam.us` returns Cloudflare addresses; `curl -s https://mm.carterfam.us/ | grep -c 'Jazz, Gin'` is 1; `curl -s -o /dev/null -w '%{http_code}' https://mm.carterfam.us/evidence/round2-notebook.jpg` is 404; `curl -s -o /dev/null -w '%{http_code}' https://mm.carterfam.us/admin` is 401.
- [ ] **Step 4:** Hand over to the owner: the URL, where the password file is (not its contents), and the phone checks from the spec's verification list.
