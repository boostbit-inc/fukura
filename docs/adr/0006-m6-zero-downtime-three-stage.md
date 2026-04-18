# ADR 0006: Rename the `team` privacy enum to `org` via a 3-stage deploy

Date: 2026-04-18
Status: Accepted

## Context

The spec's privacy tiers are `private` / `org` / `public` (§4). The
hub's SQL enum is `('private', 'team', 'public')`. The rename from
`team` to `org` is a breaking change for anything that reads the
column by value. Single-release approaches run into a window where
either the app or the DB is ahead of the other: if the DB is updated
first, old app instances see an unknown enum and 500; if the app is
updated first, it writes `'org'` into a column that does not accept
it.

The hub currently has no fleet-rolling guarantee — we cannot assume
an atomic restart across all replicas.

## Decision

Roll the rename out in three releases, with a DB step between the
first and the third. At no point is the surface inconsistent with
the DB state.

### Stage 1 — Both-accept

Deploy an app release that:

- Reads `team` and `org` as the same logical value (`org`).
- Writes `org` internally; a legacy path still accepts `team` on
  input and normalises.
- Responses always say `org`.

Requires DB to hold both values in the enum (
`ALTER TYPE privacy_level ADD VALUE 'org'`). No row rewrite yet.

Sit on this release long enough to confirm stability (hours — days
depending on traffic).

### Stage 2 — Backfill

Batch `UPDATE notes SET privacy = 'org' WHERE privacy = 'team'` in
chunks of 10 000 rows. No app change. If the job fails halfway the
fleet still reads both values correctly (per Stage 1 behaviour).

### Stage 3 — Single-accept

Deploy an app release that:

- Rejects `team` on input (returns 400 or logs as anomaly).
- Only reads `org`.

Then run the DB cleanup to drop `'team'` from the enum. Because
Postgres does not allow dropping an enum value directly, the pattern
is: create a new enum `privacy_level_v2 AS ENUM('private','org','public')`,
swap the column type, drop the old enum.

## Consequences

- Three deploys instead of one; schedulable by the maintainer rather
  than dictated by a surprise failure.
- Rollback at any stage rolls back cleanly:
  - Stage 1 rollback: reverts the app, no DB change needed.
  - Stage 2 rollback: some rows are already `org`, which Stage 1
    reads correctly. No rewrite needed.
  - Stage 3 rollback: re-deploy the Stage 1 app; the DB still holds
    only `org` values, which Stage 1 handles.
- The contract test `/v1/notes` returning `privacy=team` never fires
  after Stage 1; the real change point for CLI / MCP callers is
  Stage 1.
- Each stage is logged in `docs/implementation-status.md` as its
  code lands; operators running forks follow the same sequence.
