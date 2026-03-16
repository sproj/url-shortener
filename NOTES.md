# URL Shortener - Current Notes - 16 March 2026

## Current snapshot

Completed and working:

- `App` and `TestApp` are both builder-driven and much less tangled than before.
- App startup is now composed around injected infra:
  - config load,
  - Postgres pool creation,
  - optional Redis connection,
  - app/service wiring,
  - server start.
- Shared test infra is in place for both Postgres and Redis:
  - `tests/common/shared_container.rs`
  - `tests/common/test_db.rs`
  - `tests/common/test_redis.rs`
- `ShortUrlService` now supports:
  - create with retry-on-code-conflict,
  - list/get/delete,
  - redirect decision resolution,
  - redirect cache read-through,
  - cache invalidation on delete.
- Redis-backed redirect cache is implemented:
  - `RedirectCache` trait
  - `RedirectCacheChecker`
  - `NoopRedirectCache`
- Tracing replaced most ad hoc logging (`println!`, `dbg!`, `eprintln!`) in app code.
- Startup/runtime error handling has improved:
  - env parsing now returns `StartupError::Config`
  - server listen/start paths return `Result`
  - fewer panic paths in non-test code.

## Existing integration coverage

Green suites already present:

- `health_test`
- `ready_test`
- `shorten_tests`
- `retry_on_conflict_test`
- `redirect_tests`
- `error_tests`

Covered behavior includes:

- readiness success/failure
- create/list/get/delete short URLs
- input validation/error responses
- redirect matrix (`301/302/307/308/404/410`)
- retry success and retry exhaustion on code conflict

## Immediate next work

1. Add integration tests for Redis cache behavior.
2. Cover both paths explicitly:
   - cache hit for redirect lookup
   - cache invalidation after delete
3. Keep tests end-to-end:
   - use real Redis container via `TestAppBuilder`
   - avoid unit-only cache tests for the first pass

## Suggested cache test cases

1. `resolve_redirect_decision` caches a permanent redirect:
   - create short URL
   - hit `/r/{code}` once to populate cache
   - remove the row from Postgres manually
   - hit `/r/{code}` again
   - expect redirect still succeeds from cache

2. deleting a cached code invalidates Redis:
   - create short URL
   - hit `/r/{code}` once to populate cache
   - delete via API
   - hit `/r/{code}` again
   - expect `404` or `410` according to current delete semantics, but not cached redirect

3. cache miss still falls back to DB when Redis is enabled:
   - create short URL
   - first redirect should succeed without pre-seeded cache
   - useful mainly as a smoke test for Redis wiring

## Near-term cleanup

1. Remove DB `id` from API responses and make `uuid` the public identifier.
2. Replace RPC-style `GET /shorten/getByCode/{code}` with the final API shape.
3. Continue removing remaining `unwrap`/panic paths in non-test code.
4. Add more focused startup/config failure tests now that startup returns `Result` more consistently.
