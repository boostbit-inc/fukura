# ADR 0005: Existing `privacy=private` rows are kept on the server

Date: 2026-04-18
Status: Accepted (confirmed 2026-04-18 by maintainer)

## Context

The spec says `private` notes MUST stay client-local and the server
MUST reject uploads with that tier (§4). The hub, however, has
accepted `privacy=private` uploads from day one and those rows exist
in production today — authored by real users, visible only to their
authors. Three options:

a. **Keep them**: new uploads are rejected, but pre-existing rows
   stay in place and remain visible to their authors. The spec's
   "server MUST reject" applies prospectively.
b. **Promote them to `org`**: rewrite `private` → `org`, broadening
   visibility.
c. **Delete them**: reclaim the storage, losing the data.

## Decision

Option (a). When the server's `privacy=private` rejection lands
(P3 Slice 5, `422 Unprocessable Entity`), existing rows are left
untouched. The server owner may choose to run a one-off purge later
if they want strict spec-compliance at rest, but the default position
is preservation.

Rationale:

- The users who uploaded those rows intended them to be author-only.
  Promoting to `org` violates that intent. Deleting destroys user
  data without warning.
- The *migration* of data to a different tier is a different decision
  than the *policy change* that new private uploads are refused. We
  make the policy change now and defer the migration indefinitely.
- Prospective enforcement makes the spec true for all uploads after
  Slice 5, which is all that callers relying on spec semantics need.

## Consequences

- `GET /v1/notes?privacy=private` is not part of the spec; existing
  rows are only reachable through the author's `/api/*` UI paths,
  which is where they were always read from.
- Storage metric: a small legacy slice of the `notes` table carries
  the `private` enum value indefinitely. Dashboards that grouped by
  privacy will continue to show that bucket until purged.
- If a strict-compliance audit ever requires zero `private` rows,
  the cleanup is a single `DELETE` batch with a clear policy trail
  (this ADR + the audit ticket), rather than an implicit data
  rewrite.
- Client behaviour is unaffected: the client-side rejection in
  fukura v0.4 Task 3 already ensures no new `private` uploads can
  leave a conformant client.
