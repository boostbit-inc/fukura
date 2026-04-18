# ADR 0003: `/v1/` and `/api/` coexist on the same hub

Date: 2026-04-18
Status: Accepted

## Context

The hub server already serves 33 endpoints under `/api/*` for auth
flow, note CRUD + comments + likes, organisations, users, tags, and
dashboards. Most of these are shaped for the web UI, not for CLI /
agent clients, and they are not covered by `docs/fukurahub-api.md`.

Meanwhile, the spec reserves `/v1/*` for a small, deliberately
narrow surface: note upload, note fetch, note search, attempts batch,
attempts stats, health, info. CLI clients program against this
surface; web UI clients do not care about it.

Two options were considered:

1. Collapse everything under `/v1/*`, rewriting the UI backend.
2. Rewrite the UI backend under `/api/*`, ship a parallel `/v1/*`.
3. Do both — `/v1/*` for spec-conformant clients, `/api/*` for the
   web UI's richer surface.

## Decision

Option 3. Run both prefixes side-by-side on the same binary, backed
by the same database.

- `/v1/*` implements `docs/fukurahub-api.md` verbatim. CLI and MCP
  clients use only this. Every behaviour is contract-tested against
  `fukura/tests/hub_http_client.rs`.
- `/api/*` carries the UI's needs — session cookies, multi-field
  note bodies, comment threads, dashboards. It evolves without spec
  constraints.

Both layers share handler bodies for the overlapping operations
(note write, note read). `/api/notes` writes rows that `/v1/notes`
reads. Where the wire shapes differ (e.g. the spec returns
`{object_id, url, ontology}` but `/api/notes` returns a richer
object), adapters at the edge convert.

Auth: `/v1/*` requires Bearer only; `/api/*` accepts Bearer + cookie
session. The middleware is shared.

## Consequences

- The hub's route count grows rather than shrinks. This is fine; both
  surfaces have first-class callers.
- Duplicate test coverage: `/v1/*` is exercised by contract tests
  imported from fukura; `/api/*` by the hub's own integration tests.
- Deprecation story: if a `/api/*` endpoint later gets a spec
  counterpart, the `/api/*` side can be marked deprecated but not
  removed — third-party scripts may depend on it.
- `/api/notes` writes land in the same table. `/v1/notes` must still
  return them cleanly, treating missing `ontology` as
  `category = "generic.unknown"` (see alignment §2).
