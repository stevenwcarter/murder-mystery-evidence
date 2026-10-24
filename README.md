# murder-mystery-evidence

Small Rust/Axum site for the 2026-10-24 murder-mystery party. It shows the
evidence photos (embedded in the binary) round by round; the host advances the
round from an admin page and guests' pages update live over SSE.

## Configuration

| Variable | Default | Meaning |
|---|---|---|
| `MM_ADMIN_PASSWORD` | required | Password for the `/admin` pages (any user name) |
| `MM_INITIAL_ROUND` | `1` | Round shown at start |
| `MM_LISTEN` | `0.0.0.0:8080` | Listen address (also probed by `healthcheck`) |
| `RUST_LOG` | `info` | Log filter |

## Run

    MM_ADMIN_PASSWORD=pw cargo run

`murder-mystery-evidence healthcheck` probes `MM_LISTEN` and exits 0 or 1.

## Build and push

    docker build -t docker.wind-halfbeak.ts.net/murder-mystery-evidence:0.1.0 . && docker push docker.wind-halfbeak.ts.net/murder-mystery-evidence:0.1.0

## See also

- Design: `docs/superpowers/specs/2026-10-24-evidence-site-design.md`
- Deployment manifests: `~/k8s-config/kubernetes/apps/murder-mystery/`
