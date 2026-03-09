# URL Shortener - Current Notes - 3rd March 2026

## Current status

Completed:

* Split startup/runtime concerns (`build`, `listen`, `serve`/`run`) and removed startup config from request state.
* Added Postgres pool integration (`deadpool-postgres`) with startup wiring.
* Added readiness endpoint (`/ready`) that returns `200` when DB is reachable and `503` otherwise.
* Added container-backed readiness tests (positive and negative cases).
* Added first migration and `short_url` vertical slice:
* repository insert/list,
* POST create route,
* GET list route,
* successful end-to-end manual test.
* Added `GET /shorten/{id}` and `DELETE /shorten/{id}` with integration coverage for success and `404` paths.
* Added request/response transport models for create flow and a validated `ValidatedCreateShortUrlRequest` conversion path.
* Persisted validated create input correctly (`expires_at` and normalized `long_url`).
* Improved validation error responses (stable top-level message, structured per-field details).
* Added integration tests for create-path validation rules (scheme, password, past expiry, empty/too-long input).

## Next session priorities

1. **Code generation strategy (start here tomorrow)**
   * Replace `bs58(long_url)` placeholder with bounded short-code generation.
   * Decide collision handling policy (retry loop + unique DB constraint on `code`).
   * Decide deduplication policy:
   * always create new code, or
   * reuse existing active code for same normalized input.

2. **Redirect endpoint**
   * Add `GET /r/{code}` lookup path.
   * Define response behavior:
   * `302/307` (temporary) vs `301/308` (permanent),
   * missing/deleted/expired code status and body.
   * Add integration tests for success + not found + expired/deleted branches.

3. **Coverage quick wins (5 tests)**
   * `short_url_service`: add test for non-`code` conflict branch (no retry, bubble storage error).
   * `short_url_service`: add test for retry exhaustion branch (already added, keep as baseline when refactoring).
   * `short_url_handlers`: add integration test for `GET /shorten` empty list path.
   * `short_url_handlers`: add integration test for `GET /shorten/{id}` with non-numeric id (path parse failure/error shape).
   * `app/config/startup`: add focused tests for config-load failure and pool-creation failure mapping to `StartupError`.

4. **API identifier cleanup (after redirect pass)**
   * Current temporary state: `get by code` is exposed in RPC style for momentum while redirect feature is in progress.
   * Remove database `id` from API request/response shapes.
   * Make `uuid` the canonical external resource identifier.
   * Remove `/shorten/{id}` and migrate handlers/tests to uuid-based lookups.
   * Revisit endpoint shape toward REST conventions:
   * canonical path lookup by uuid,
   * optional query/filter lookups by `code`/`uuid` where appropriate.

## Coverage baseline (llvm-cov)

* Function: `64.66%` (`75/116`)
* Line: `76.77%` (`499/650`)
* Region: `72.02%` (`623/865`)

## Deferred TODO

1. **Config ergonomics for tests**
   * Current misconfiguration tests require full `Config` literals, including unrelated fields.
   * Keep this for momentum now.
   * Revisit later with goals:
   * one source of truth for config loading,
   * targeted test overrides for relevant fields only,
   * explicit typed config errors instead of panic-based env parsing.
