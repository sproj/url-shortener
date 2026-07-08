# url-shortener

A production-shaped URL shortener built as a small distributed system, written in Rust. It started as a single Axum service and was deliberately decomposed into a multi-crate workspace to practice the architectural patterns of a real microservice deployment: an API gateway that owns identity, an event-driven analytics pipeline, shared caching, and full observability — all deployed to Kubernetes via CI/CD.

This is a personal learning project. The goal was to build the full arc from application code to a running cluster, evidence-driven at each step (load test numbers, cache hit ratios, trace waterfalls) rather than received wisdom.

## Architecture

```
                    ┌──────────────┐
   client ────────▶ │  api-gateway │  (identity: login/logout/refresh, user CRUD)
                    │              │  issues + validates JWTs
                    └──────┬───────┘
                           │ reverse proxy (prefix-matched, JWT pass-through)
                           ▼
                    ┌──────────────┐        ┌───────────┐
                    │ url-shortener│──cache─▶│   Redis   │
                    │              │         └───────────┘
                    └──────┬───────┘
                           │ publishes RedirectEvent
                           ▼
                    ┌──────────────┐        ┌──────────┐
                    │  RabbitMQ    │───────▶│analytics- │
                    │ (+ DLX/DLQ)  │        │ consumer  │
                    └──────────────┘        └─────┬─────┘
                                                    ▼
                                              Postgres (analytics DB)
```

- **`api-gateway`** is the single entry point behind the ingress. It owns login/logout/refresh, user records, and JWT issuance, and reverse-proxies everything else to the appropriate upstream by longest-prefix path match (`crates/api-gateway/src/api/proxy_handler.rs`). It validates the bearer token if one is present and passes the request through — it does not require auth for routes that don't need it.
- **`url-shortener`** owns short URL creation, lookup, and redirect resolution. It trusts JWTs signed by `api-gateway` (shared secret) rather than looking users up itself — ownership checks are a pure in-memory UUID comparison against the token's `sub` claim, no DB round trip on the hot path.
- **`auth`** is a shared library crate (JWT encode/decode, role claims, Argon2 password hashing) used by both `api-gateway` and `url-shortener`.
- **`common`** holds cross-crate primitives: config loading, shared error types, and the `RedirectEvent` message contract published to RabbitMQ and consumed by `analytics-consumer`.
- **`analytics-consumer`** is an independent service that subscribes to redirect events and persists them to a separate Postgres database, decoupled from the request path — a redirect is never slowed down by analytics writes. Failed messages are nacked to a dead-letter exchange/queue rather than dropped.

### Why split it this way

The url-shortener → api-gateway split was driven by one constraint: the redirect path is the hottest path in the system and is cached in Redis. An upsert-on-every-request pattern (looking up or creating a user record on each authenticated call) would have put a DB round trip in front of that cache on every request. Instead, `url-shortener` stores the JWT `sub` (a UUID) directly as `user_id` with no foreign key and no local users table — ownership is a claims comparison, not a lookup.

## Request lifecycle: a redirect

`ANY /r/{code}` on `url-shortener`:

| Condition | GET | non-GET |
|---|---|---|
| Code not found | 404 | 404 |
| Deleted | 410 | 410 |
| Expired | 410 | 410 |
| No expiry (permanent) | 301 | 308 |
| Future expiry (temporary) | 302 | 307 |

The decision is computed once (`ShortUrlService::resolve_redirect_decision`), the resolved target is cached in Redis (invalidated on delete), and a `RedirectEvent` is published to RabbitMQ asynchronously via `tokio::spawn` so a slow or unavailable broker never adds latency to the response.

## Tech stack

- **Language / runtime:** Rust, Tokio, Axum
- **Data:** Postgres (via `tokio-postgres` + `deadpool-postgres`), `refinery` migrations embedded at compile time
- **Cache:** Redis (`deadpool-redis`)
- **Messaging:** RabbitMQ (`lapin`), with a dead-letter exchange/queue for poison messages
- **Auth:** JWT (`jsonwebtoken`), Argon2 password hashing
- **Observability:** OpenTelemetry traces exported to Jaeger, Prometheus metrics (`metrics` + `axum-prometheus`) scraped via a `ServiceMonitor`, Grafana dashboards
- **API docs:** OpenAPI/Swagger UI (`utoipa`)
- **Load testing:** k6, run as a Kubernetes Job against the live deployment
- **Infra:** Kubernetes manifests per namespace, GitHub Actions CI/CD, images published to GHCR

## Observability

Every request is traced end-to-end — handler → service → repository → cache — and exported over OTLP/gRPC to Jaeger. Prometheus metrics cover both RED (rate/errors/duration on redirects and short-URL operations) and USE (Redis cache hit/miss counters, DB pool utilization) signals, visualized in a Grafana dashboard (`k8s/dev/olly/grafana.dashboard.json`). Locally: ~12k req/s on a cached redirect, p50 ~2.5ms / p99 ~11ms (`wrk`, single machine).

## Testing

- **Unit tests** for service-layer logic against in-memory mock repositories/traits — no I/O.
- **Integration tests** (`testcontainers`) spin up real Postgres and Redis containers per crate and exercise the full HTTP stack, including auth, redirect resolution, retry-on-code-collision, and cache invalidation.
- Each crate (`url-shortener`, `api-gateway`, `auth`, `common`, `analytics-consumer`) has its own CI workflow, path-filtered so a change to one crate doesn't trigger a full-workspace build. A separate workspace-level check runs when `Cargo.toml`/`Cargo.lock` change.

```bash
cargo build --workspace
cargo test --workspace                # unit + integration (requires Docker)
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

## CI/CD

GitHub Actions, one check-and-test workflow and one publish workflow per crate:

- **Check & test** — lint → clippy → fmt gate, then unit and integration tests in parallel, triggered only when that crate's source (or its local dependencies) changed.
- **Publish** — on push to `main` or a `v*` tag, builds and pushes a Docker image to `ghcr.io/sproj/<crate>` with GitHub Actions layer caching.

## Deployment

Kubernetes manifests live under `k8s/dev/`, one directory per namespace (`url-shortener`, `api-gateway`, `analytics`, plus shared observability). Secrets are synced via External Secrets rather than committed. Redis and RabbitMQ run as shared cluster-level services rather than per-app instances. `api-gateway` fronts the ingress and is the only service with a public route; `url-shortener` and `analytics-consumer` are ClusterIP-only, reachable exclusively through the gateway's reverse proxy.

Current infrastructure target is a small self-hosted k8s cluster on AWS EC2 (provisioned by a separate Terraform repo); this repo's only contract with that infrastructure is "produce an image + manifests," and has no opinions about how the cluster itself is built.

## Local development

```bash
docker compose up -d       # Postgres, Redis, RabbitMQ, Jaeger, RedisInsight
cargo run -p url-shortener # or -p api-gateway / -p analytics-consumer
```

Each binary loads its config from `.env` (`.env.test` under `ENV_TEST=1`); see `.env.example` for the full variable list per service.

## Repository layout

```
crates/
├── url-shortener/      short URL CRUD, redirect resolution, Redis cache, RabbitMQ publisher
├── api-gateway/         identity, JWT issuance, reverse proxy to upstream services
├── auth/                 shared JWT + password hashing library
├── common/                shared config, error types, event contracts
└── analytics-consumer/    RabbitMQ consumer, persists redirect events
k8s/dev/                  per-namespace Kubernetes manifests
load_tests/                k6 load test + Kubernetes Job
.github/workflows/         per-crate CI + publish pipelines
```
