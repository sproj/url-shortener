# URL Shortener - Current Notes - 1 April 2026

## Current snapshot

Completed and working:

- Public `short_url` API is UUID-based; DB `id` is not exposed.
- Short URL HTTP surface currently includes:
  - `POST /shorten`
  - `GET /shorten`
  - `GET /shorten/{uuid}`
  - `DELETE /shorten/{uuid}`
  - redirect path `/r/{code}`
- Redirect behavior is implemented and integration-tested:
  - `301` / `308` for permanent redirects
  - `302` / `307` for temporary redirects
  - `404` for missing code
  - `410` for expired or deleted code
- Redis-backed redirect caching is implemented:
  - read-through on redirect lookup
  - cached redirect survives DB row removal
  - cache invalidates on delete
  - cache-disabled fallback via `NoopRedirectCache`
- Retry-on-code-conflict behavior remains implemented and tested.

## User and auth work now present

- `users` table exists and `short_url.user_id` ownership column exists in migrations.
- User vertical slice exists:
  - create user
  - list users
  - get user by UUID
  - soft delete user by UUID
  - update password
- Password hashing + salt generation are in place.
- Auth HTTP surface now exists and works:
  - `POST /login`
  - `POST /logout`
  - `POST /refresh`
- Login issues:
  - short-lived access token
  - longer-lived refresh token
- Refresh flow is implemented:
  - refresh token is cached in Redis
  - refresh rotates tokens
  - old refresh token is revoked
- Logout flow is implemented:
  - cached refresh token is revoked
- Access/refresh token type validation is enforced in extractors.

## Test picture

Green suites present:

- `health_test`
- `ready_test`
- `shorten_tests`
- `redirect_tests`
- `retry_on_conflict_test`
- `users_tests`
- `auth_tests`
- `error_tests`

Coverage/testing status:

- redirect/cache behavior is covered well
- login, logout, and refresh flows are integration-tested
- startup/config/database/redis error paths have focused unit tests
- shared integration-test helper noise from `dead_code` was intentionally allowed in `tests/common`

## Current code quality state

- App/test startup has been untangled substantially.
- Shared Postgres and Redis test containers are in place.
- `tracing` replaced ad hoc prints in app code.
- `unwrap` audit is effectively complete for non-test runtime paths.
- RedisInsight is now in local `docker-compose.yaml` for cache inspection during development.

## Immediate next focus

1. Add protected routes using the existing access-token extractor foundation.
2. Decide and implement ownership enforcement for user-owned short URLs.
3. Add vanity URL support.
4. Define collision/conflict policy for vanity codes:
   - global uniqueness
   - reserved values
   - ownership rules
   - update/delete behavior
5. Add tests for:
   - protected route requires valid access token
   - wrong token type on protected route
   - user cannot mutate another user's owned short URL
   - vanity code conflict returns expected error

## Secondary follow-up

1. Deployment work can resume once auth/ownership shape is settled.
2. Finalize Kubernetes manifests once runtime secret/config requirements stop moving.
3. Consider cleaning up `AppBuilder` overrides that are currently misleading:
   - `with_state(...)` is unused
   - `with_max_retries(...)` does not currently override `AppState.max_retries`
