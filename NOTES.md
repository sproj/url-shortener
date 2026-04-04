# URL Shortener - Current Notes - 3 April 2026

## Current snapshot

Completed and working:

- Public `short_url` API is UUID-based; DB `id` is not exposed.
- Short URL HTTP surface currently includes:
  - `POST /shorten` (protected)
  - `POST /shorten/vanity` (protected)
  - `GET /shorten` (admin only)
  - `GET /shorten/{uuid}` (owner or admin)
  - `PATCH /shorten/{uuid}` (owner or admin)
  - `DELETE /shorten/{uuid}` (owner or admin)
  - redirect path `/r/{code}`
- Vanity URL creation is implemented and integration-tested.
- Vanity URL update is implemented and integration-tested.
- Ownership checks are now present across the protected short URL metadata routes.
- Anonymous short URLs are treated as admin-only for metadata read/delete paths.
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
  - create user (open)
  - list users (admin only)
  - get user by UUID (self or admin)
  - soft delete user by UUID (self or admin)
  - update password (self or admin)
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
- Authenticated user context is now used by vanity URL create/update flows.
- Access-token protection has now been extended to most user and short URL routes.

## Test picture

Green suites present:

- `health_test`
- `ready_test`
- `shorten_tests`
- `redirect_tests`
- `retry_on_conflict_test`
- `users_tests`
- `auth_tests`
- `vanity_tests`
- `update_tests`
- `error_tests`

Coverage/testing status:

- redirect/cache behavior is covered well
- login, logout, and refresh flows are integration-tested
- vanity creation/update flows are integration-tested
- protected-route authorization paths are integration-tested for users and short URLs
- startup/config/database/redis error paths have focused unit tests
- shared integration-test helper noise from `dead_code` was intentionally allowed in `tests/common`

## Current code quality state

- App/test startup has been untangled substantially.
- Shared Postgres and Redis test containers are in place.
- `tracing` replaced ad hoc prints in app code.
- `unwrap` audit is effectively complete for non-test runtime paths.
- RedisInsight is now in local `docker-compose.yaml` for cache inspection during development.

## Immediate next focus

1. Decide whether any remaining routes should stay open beyond:
   - `POST /users`
   - public redirect `/r/{code}`
2. Revisit vanity code policy details:
   - reserved values
   - update semantics
   - delete/reuse semantics
   - any admin override behavior


