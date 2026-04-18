# ADR 0007: P3 Slice 1 — add the `attempts` table

Date: 2026-04-18
Status: Proposed

## Context

Hub API §5.2 requires `POST /v1/attempts` (batch upload of solution
attempts) and `GET /v1/attempts/stats` (per-fingerprint aggregation).
The CLI side is already implemented and `/v1/attempts` sync is wired
into `fuku hub sync-attempts`, but the server has no table and no
route. Until it lands, the CLI uploads to a 404 and attempt data
never aggregates across machines.

Slice 1 is the lowest-risk piece of P3: a new table, two routes, no
existing-data migration. It is the right place to start because:

- Adding a table does not change any existing behaviour, so it can
  ship before the rest of P3 without coordinating with Slice 4's
  enum rename or Slice 3's `/v1/` route move.
- The contract test exercises it immediately; the two failing cases
  (`POST /v1/attempts` and `GET /v1/attempts/stats`) go green without
  touching anything else.
- Aggregate effectiveness stats are the most visible user benefit and
  motivate the rest of the rollout for the maintainer.

## Decision

In a single hub PR, land:

1. A new migration `backend/migrations/002_attempts.sql` adding the
   `attempts` table exactly as described in `fukurahub-alignment.md`
   M8.
2. A new handler module `backend/src/api/attempts.rs` with:
   - `POST /v1/attempts` — accepts an `AttemptsBatch`, returns
     `202 Accepted` + `{accepted, rejected, errors}`. `attempt_id`
     UNIQUE constraint provides idempotency (spec §6).
   - `GET /v1/attempts/stats` — aggregates by `fingerprint`, supports
     optional `fingerprint=` filter and `since=` timestamp.
3. Router wiring in `backend/src/main.rs` — both routes under `/v1/`
   alongside the existing `/api/` routes (ADR 0003).

`implementation-status.md` rows move for §5.2 hub server from 🚧 to ✅.
Contract test job in fukura-hub CI stays in snapshot mode; the two
`/v1/attempts*` cases flip to passing after this PR merges.

## Consequences

- First `/v1/*` endpoints land on the hub, establishing the pattern
  the other slices follow: route handlers read their inputs as
  spec-shaped DTOs, return spec-shaped responses, and share DB
  access with the `/api/*` layer where relevant.
- The contract test CI job begins catching regressions in these two
  endpoints immediately. Any future change that breaks
  `POST /v1/attempts` wire contract fails the job.
- The `attempts` table has no foreign key to `notes.object_id` — the
  spec treats fingerprints as the join key and allows attempts to
  exist for notes that were never uploaded to this hub. Keep it that
  way; adding FKs would couple Slice 1 to Slice 3 (object_id-based
  lookup) unnecessarily.
- Slices 2–5 build on this pattern. Slice 4's 3-stage enum rename
  (ADR 0006) still requires maintainer tempo and is not unlocked by
  Slice 1.
