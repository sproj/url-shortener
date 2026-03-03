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
* Added request/response transport models for create flow and a validated `NewShortUrlDto` conversion path.
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

3. **Validation/error follow-up**
   * Keep `ApiError.message` short/stable and per-field issues in `detail`.
   * Add malformed JSON/body-shape test for create path.
   * Decide whether to keep all validation-related failures as `400` for now.
   * Revisit `2048` max URL length and move limit to config if still needed.

4. **Boundary cleanup follow-up**
   * Continue keeping domain models free of API transport/error dependencies.
   * Keep HTTP mapping (`*Error -> ApiError`) at API boundary.

5. **Type/mapping hardening follow-up**
   * Keep fallible row mapping (`try_get` + mapping errors).
   * Verify DB/Rust numeric alignment (`INTEGER -> i32`, `BIGINT/BIGSERIAL -> i64`).

## Deferred TODO

1. **Config ergonomics for tests**
   * Current misconfiguration tests require full `Config` literals, including unrelated fields.
   * Keep this for momentum now.
   * Revisit later with goals:
   * one source of truth for config loading,
   * targeted test overrides for relevant fields only,
   * explicit typed config errors instead of panic-based env parsing.
