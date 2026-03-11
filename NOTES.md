# URL Shortener - Current Notes - 10 March 2026

## Current snapshot

Completed and working:

- App startup is now builder-based (`App::builder().build().await?.start().await`).
- `App`/`AppBuilder` own config loading, DB pool creation, migration, and server start flow (no free startup functions).
- `TestApp` is now builder-based and composable (`TestApp::builder()` with config/db/state-builder/migration toggles).
- Startup/config panic cleanup is in progress and mostly done:
  - config env reads/parse now return `StartupError::Config`.
  - server listen/start now return `Result<_, StartupError>`.
- Logging switched from `println!/eprintln!/dbg!` to `tracing` across the app path.
- Short URL create/list/get/delete flows are in place and integration-tested.
- Redirect flow is implemented and integration-tested:
  - `/r/{code}` returns:
  - `301` for GET + permanent code (no expiry),
  - `308` for non-GET + permanent code,
  - `302` for GET + temporary code (future expiry),
  - `307` for non-GET + temporary code,
  - `404` when code missing,
  - `410` when expired or deleted.
- Retry-on-code-conflict behavior is implemented and covered by integration tests (success + exhausted retries).

## Current test status

- Integration suites present:
  - `health_test`
  - `ready_test`
  - `shorten_tests`
  - `retry_on_conflict_test`
  - `redirect_tests`
  - `error_tests`
- Redirect matrix and collision/retry matrix are both green.

## Next feature focus (tomorrow)

1. Add Redis cache for redirect lookups (`code -> redirect decision/long_url`).
2. Define cache policy before coding:
   - key shape and serialization format,
   - TTL strategy for temporary links,
   - invalidation strategy for delete/update/expiry transitions.
3. Keep DB as source of truth; cache should be read-through/write-through (or read-through + explicit invalidation).
4. Add integration coverage for cache-hit/cache-miss behavior and stale-entry safety.

## Near-term cleanup

1. Remove DB `id` from API responses and converge on `uuid` as public identifier.
2. Replace remaining RPC-style `/shorten/getByCode/{code}` with final REST shape (after redirect/cache settles).
3. Continue removing remaining `unwrap`/panic paths in non-test code.
4. Add tracing spans/request IDs once Redis is introduced.
