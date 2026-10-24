# Murder-mystery evidence site — design

Date: 2026-10-24. Status: approved in conversation (ship-it --ask); built the
same day for a party that evening.

## Intent

Tonight's party plays *Jazz, Gin, Money & Murder* (a Masters of Mystery
prefab, 1920s Great Gatsby theme). The host plays the detective and, at the
start of rounds 2, 3 and 4, reads a script announcing newly uncovered
evidence. Guests then discuss the evidence to find the murderer. There is one
set of printed clues per round, which has to be passed around; this site puts
the same images on every guest's phone.

The site shows the evidence for the **current round and every earlier
round**, nothing later. The host advances the round from their phone and
every open guest page updates by itself within about a second, so clue cards
appear while the detective is still talking.

The site runs for one evening. Simplicity and "works on the first try" beat
reuse, polish of the admin side, and operational durability.

## Goals

- Public page at `https://mm.carterfam.us`: no login, no code to enter, one
  QR code / address for the whole evening.
- Guest page lists revealed evidence as cards, newest round first within the
  page. Tapping a card opens a full-screen viewer with pinch-to-zoom and pan;
  every image has a download button.
- Host page at `/admin`, protected by a password, with one button per round;
  the current round is highlighted. Setting a round (forward or back) takes
  effect for all guests immediately.
- Locked rounds are *not* obtainable by URL guessing: the image endpoints
  refuse files from rounds that are not yet revealed.
- Theming matches the decorations: black and gold Art Deco (geometric double
  line frames, fan and sunburst motifs, gold display type), with a quiet
  autumn/Halloween accent at most.

## Non-goals

- Accounts, multiple hosts, per-round toggles, scheduling.
- Persisting the round across pod restarts (see "State").
- Thumbnails or image processing: the images are 1500–1800 px JPEGs, 220–700
  KB each, 1.7 MB in total. Phones load them directly.
- Keeping the deployment after the party. It is torn down the next day
  (TODO item in `k8s-config`).

## Anti-goals

- A guest seeing round 3 or 4 evidence before the detective announces it.
  That would spoil the game, so gating is enforced server-side, not by hiding
  cards in the DOM.
- Guests needing to refresh or rescan anything between rounds.
- A build that needs debugging tonight (wasm, hydration, cargo-leptos). Hence
  plain server-rendered HTML with a few lines of JavaScript.

## The evidence

Source files live in `Evidence/` in this repo and are embedded in the binary
as they are (no copies, no renames). Round 1 has no evidence; the game has
four rounds.

| Round | File | Title on the card |
|---|---|---|
| 2 | `Round 2 - Notebook.jpg` | The Notebook |
| 3 | `Round 3 - Note.jpg` | The Note |
| 4 | `Round 4 - View 1.jpg` | The Study, view 1 |
| 4 | `Round 4 - View 2.jpg` | The Study, view 2 |

The manifest is a static table in Rust (`round`, `slug`, `title`,
`file name`). Slugs are the public names (`round2-notebook.jpg`,
`round3-note.jpg`, `round4-view-1.jpg`, `round4-view-2.jpg`). A test asserts
every manifest entry resolves to an embedded file.

## How it is made

A single Rust binary, following `~/.claude/rust-crate-decisions.md`:

- `axum` 0.8, `tokio`, `tower-http` (compression), `clap` derive with a
  `serve` default and a `healthcheck` subcommand, `tracing` +
  `tracing-subscriber` with `EnvFilter` (default `info`), `rust-embed` for
  `Evidence/` and `assets/` (CSS, JS), `maud` for HTML (chosen over
  minijinja: four small pages, compile-time checked, no runtime template
  loading), `subtle` for constant-time password comparison, `base64` for the
  Basic header, `tikv-jemallocator` under `cfg(target_env = "musl")`, edition
  2024, `rustfmt.toml` with edition 2024, release profile `lto`,
  `codegen-units = 1`, `opt-level = 3`. No TLS anywhere in the graph (the
  gateway terminates TLS); `cargo tree -i openssl` and `-i native-tls` are
  empty.
- Docker: `blackdex/rust-musl:x86_64-musl` builder with a stub-crate
  dependency layer, `FROM scratch` runtime, non-root `USER 65532`,
  `EXPOSE 8080`, `HEALTHCHECK CMD ["/murder-mystery-evidence", "healthcheck"]`.
- Tests: `cargo test` with `axum-test` for handler-level tests; `cargo clippy
  --all-targets -- -D warnings` and `cargo fmt --check` clean.

### Configuration (environment)

| Variable | Default | Meaning |
|---|---|---|
| `MM_LISTEN` | `0.0.0.0:8080` | bind address |
| `MM_ADMIN_PASSWORD` | required to serve | Basic-auth password for `/admin` (any user name) |
| `MM_INITIAL_ROUND` | `1` | round at startup (1 = nothing revealed) |
| `RUST_LOG` | `info` | tracing filter |

Missing `MM_ADMIN_PASSWORD` is a startup error, so a misconfigured deploy
fails loudly rather than serving an unprotected admin page.

### State

`current_round: u8` in `1..=4`, held in a `tokio::sync::watch` channel inside
the shared app state. The sender is set by the admin handler; the SSE handler
subscribes to the receiver. The value lives in memory only: a pod restart
returns to `MM_INITIAL_ROUND`. The admin page shows the current round, so
recovering is one tap. One replica, `strategy: Recreate`, so there is never a
second pod with a different round.

### Routes

| Method, path | Auth | Behaviour |
|---|---|---|
| `GET /` | none | Guest page for the current round. `Cache-Control: no-store`. |
| `GET /evidence/{slug}` | none | The JPEG if its round ≤ current round, otherwise **404** (same body as an unknown slug, so locked and nonexistent are indistinguishable). `Cache-Control: private, max-age=3600` once served. |
| `GET /events` | none | SSE stream. Sends `event: round`, `data: <n>` immediately on connect and on every change; a comment keep-alive every 15 s so Cloudflare's idle timeout never closes it. |
| `GET /admin` | Basic | Host page: current round, one button per round 1–4. |
| `POST /admin/round` | Basic | Form field `round=<1..=4>`; sets the round, redirects (303) to `/admin`. Out-of-range → 400. |
| `GET /assets/{file}` | none | CSS and JS from the embedded `assets/`, with `Cache-Control: no-cache` and an ETag (a hash of the content), so a redeploy during the day never leaves a phone on stale CSS or JS. |
| `GET /healthz` | none | `200 ok`. Used by Kubernetes probes. |

Basic auth failures return `401` with `WWW-Authenticate: Basic
realm="detective"` so phone browsers show the native login prompt. The
password comparison is constant-time. The user name is ignored.

### Guest page

- Header: "Jazz, Gin, Money & Murder" in a gold Art Deco display face,
  subtitle "Evidence Locker" (builder may pick another period word), the
  current round ("Round 3 of 4 · the investigation continues") beneath.
- Round 1 (nothing revealed): a single framed message, "The investigation is
  underway. Evidence will appear here as the detective uncovers it."
- Revealed rounds: a section per round in **descending** order (newest at the
  top, because that is what guests opened the page to see), each with the
  round heading and a grid of cards (1 column on phones, 2 on wider screens).
  A card shows the image (`loading="lazy"`), its title, and a download link
  (`<a download href="/evidence/<slug>">`).
- Locked rounds: one dim "sealed" card per locked round ("Round 4 · sealed")
  with no item count or titles, so nothing is spoiled but guests see there is
  more to come.
- Tapping a card image opens the viewer: a full-screen `<dialog>` with the
  image, pinch-to-zoom and pan via pointer events (two-pointer scale around
  the midpoint, one-pointer pan when zoomed, double-tap toggles 1×/2.5×),
  a close button, a download button, and an "open image" link that opens the
  raw JPEG in a new tab as a fallback that uses the browser's own zoom.
  Viewport meta keeps user scaling enabled.
- Live update: `new EventSource("/events")`; when a `round` event's value
  differs from the round the page was rendered with (`data-round` on
  `<body>`), the page calls `location.reload()`. On `error` the browser
  reconnects by itself. A small gold dot in the footer shows "live" / "reconnecting".
- Works with JavaScript disabled, minus live update and the viewer (the
  image link still opens the JPEG).

### Host page

Plain and fast to use with one thumb: the title, "Current round: N", four
large buttons (Round 1 "Nothing revealed", Round 2, Round 3, Round 4), the
current one filled gold, the others outlined. Each button is its own form
posting `round=N`. Same theme as the guest page, no viewer, no SSE.

### Theme

Tokens: background `#0b0b0e` (near black), surface `#15151a`, gold
`#d4af37` with a lighter `#f1d98a` for gradients and a darker `#8a6d1f` for
hairlines, ivory text `#f3ecd8`, muted `#a89f8a`. Fonts from Google Fonts
(the site is on the public internet so guests can fetch them): a geometric
Art Deco display face for headings (`Limelight` or `Poiret One`; builder
picks what reads best at phone size) and `Cormorant Garamond` for body. Cards
and sections carry a double gold hairline frame with stepped corner
ornaments drawn in CSS (or a small inline SVG), echoing the banners. Keep
contrast high; this is read in a dim room. Subtle fan/sunburst motif behind
the header is optional. No animation beyond a fade when new cards appear.

## Deployment (in `~/k8s-config`)

`kubernetes/apps/murder-mystery/` following the repo's conventions:

- `application.yaml`: ArgoCD Application `murder-mystery`, namespace
  `murder-mystery`, sync-wave 5, automated prune + selfHeal,
  `CreateNamespace=true`, `ServerSideApply=true`.
- `manifests/murder-mystery.yaml`: Deployment (1 replica, `strategy:
  Recreate`, image `192.168.42.134:5000/murder-mystery-evidence:<version>`,
  non-root 65532, read-only root FS, drop all caps, `MM_ADMIN_PASSWORD` from
  Secret `murder-mystery-admin` key `password`, readiness/liveness on
  `/healthz`, requests 5m/16Mi, limit 64Mi) and Service `murder-mystery`
  port 80 → `http`.
- `manifests/httproute.yaml`: hostname `mm.carterfam.us` on Gateway
  `public` (namespace `gateway`), defaulted fields spelled out as in
  candyland's route. external-dns publishes the record; the Cloudflare
  certificate covers one-label names, and `mm` is one label.
- `manifests/network-policy.yaml`: default-deny like candyland (ingress from
  `ingress` and `host` entities on 8080, egress `none`).
- `manifests/admin-sealed.yaml`: SealedSecret made with
  `kubernetes/bootstrap/seal.sh murder-mystery murder-mystery-admin ...
  --from-file=password=_out/murder-mystery-admin-password`. The plain
  password is a three-word passphrase generated into `_out/`, never printed
  or committed.
- `renovate.json`: add `192.168.42.134:5000/murder-mystery-evidence` to the
  own-images automerge rule.
- Image build and push from the desktop:
  `docker build -t docker.wind-halfbeak.ts.net/murder-mystery-evidence:0.1.0 . && docker push ...`
  (the registry's two names are one store; manifests keep the LAN name).
- Docs: one bullet in `CLAUDE.md`'s app list and the app-source table, a
  short `README.md` section, and a `TODO.md` item to remove the app after
  2026-10-24.

## Verification before "done"

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test` green.
- Docker image builds; `docker run` with a password, `curl /` shows round 1
  text, `curl /evidence/round2-notebook.jpg` is 404, after `POST /admin/round
  round=2` with auth it is 200 and `/events` emitted `data: 2`.
- On the cluster: Application Synced/Healthy, `curl -H 'Host: mm.carterfam.us'
  http://192.168.42.20/` returns the page, then `https://mm.carterfam.us`
  from the internet (DNS may take a minute).
- A phone check by the owner: QR/URL opens, admin reveals round 2, the guest
  page updates without a refresh, pinch-zoom and download work.

## Invariants this feature depends on

- Gating lives in one place, `evidence::visible(round, current)`, used by
  the page renderer **and** the image handler; a handler test covers the
  locked → 404 → unlocked → 200 transition at the HTTP layer, not just the
  helper.
- The admin route is the only writer of the round; a test asserts an
  unauthenticated POST does not change it.
- Exactly one replica. A second pod would hold its own round.

## Left to the builder

- Exact display font, corner ornament drawing, copy of the sealed-card text.
- Pinch-zoom implementation details (pointer events vs. a tiny library;
  no network dependency at runtime beyond Google Fonts).
