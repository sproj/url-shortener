# URL Shortener - Current Notes - 23 March 2026

## Current snapshot

Completed and working:

- Public `short_url` API is now UUID-based; DB `id` is no longer exposed.
- RPC-style `GET /shorten/getByCode/{code}` has been removed.
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

## User/auth work now present

- `users` table and ownership column on `short_url` are in migrations.
- User vertical slice exists:
  - create user
  - list users
  - get user by UUID
  - soft delete user by UUID
  - update password
- Password handling is in place with hashing + salt generation.
- Basic login endpoint exists at root-level `/login`.
- Current login behavior verifies username + password only.
- JWT/session issuance is not yet in place.

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
- startup/config/database/redis error paths now have focused unit tests
- shared integration-test helper noise from `dead_code` was handled with `#![allow(dead_code)]`

## Current code quality state

- App/test startup has been untangled substantially.
- Shared Postgres and Redis test containers are in place.
- `tracing` replaced most ad hoc prints in app code.
- `unwrap` audit is effectively complete for non-test runtime paths.

## Next focus

Primary likely next step:

1. Add JWT to the login/logout/auth path.

Likely work items:

1. Decide token shape:
   - claims
   - expiry
   - issuer/audience
   - signing key source
2. Decide auth lifecycle:
   - login issues JWT
   - logout semantics (stateless only vs revocation/blocklist)
3. Add auth middleware/extractor for protected routes.
4. Decide whether `short_url.user_id` ownership is enforced at create/read/delete time yet.
5. Add tests for:
   - successful token issuance
   - invalid credentials
   - expired/invalid token
   - unauthorized access to protected routes

## Secondary follow-up

1. Revisit deployment work after auth shape is clearer.
2. Finalize Kubernetes manifests once runtime env and secret requirements are settled.
3. Add deployment-oriented tracing/logging polish once the app is running in-cluster.
