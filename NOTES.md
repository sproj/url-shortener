# URL Shortener - Current Notes - 17 March 2026

## Current snapshot

Completed and working:

- Public API now uses `uuid` instead of database `id`.
- `GET /shorten/getByCode/{code}` has been removed.
- Short URL HTTP surface currently includes:
  - `POST /shorten`
  - `GET /shorten`
  - `GET /shorten/{uuid}`
  - `DELETE /shorten/{uuid}`
  - `GET|POST|... /r/{code}`
- Redirect behavior is implemented and covered:
  - `301` / `308` for permanent redirects
  - `302` / `307` for temporary redirects
  - `404` for missing code
  - `410` for deleted or expired code
- Redis-backed redirect caching is in place:
  - read-through on redirect lookup
  - cache invalidation on delete
  - cache-disabled fallback via `NoopRedirectCache`
- Retry-on-code-conflict behavior remains implemented and tested.
- App/test startup has been untangled substantially:
  - shared Postgres and Redis test containers
  - composable `TestAppBuilder`
  - app wiring takes real or test collaborators cleanly
- Unit and integration coverage were improved:
  - redirect/cache integration tests added
  - startup/config/database/redis failure paths now have focused unit tests
- Logging now uses `tracing` instead of ad hoc prints in app code.

## Current test picture

Green suites present:

- `health_test`
- `ready_test`
- `shorten_tests`
- `redirect_tests`
- `retry_on_conflict_test`
- `error_tests`

Coverage is in a healthy place for the current stage:

- total line coverage is roughly `75%`
- service/repository/cache/redirect paths are better covered than startup/error formatting paths

## Next focus: deployment

Primary goal for today:

1. Deploy locally to Minikube.

Likely deployment steps:

1. Add container image build path for the app.
2. Add Kubernetes manifests for:
   - app deployment
   - app service
   - Postgres
   - Redis
   - config/secret handling
3. Decide how migrations should run in Kubernetes:
   - app startup
   - init container
   - one-shot job
4. Verify the deployed app can:
   - start cleanly
   - reach Postgres and Redis
   - serve `/health` and `/ready`
   - create and redirect a short URL end to end

## Near-term follow-up

1. Revisit the final REST shape for non-redirect lookup by `code` if needed.
2. Improve coverage in lower-value but still weak files:
   - `api/error.rs`
   - `application/app.rs`
   - `application/config.rs`
   - `application/state.rs`
3. Add deployment-oriented tracing/logging polish once the app is running in-cluster.
