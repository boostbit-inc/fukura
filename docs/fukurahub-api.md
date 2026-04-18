# Fukurahub API — v0.1 (Draft)

**Status:** Draft / Request for Comments
**License:** CC-BY 4.0
**Prior art this extends:** [EKP v1](./ekp-spec.md)

## 1. What fukurahub is

A fukurahub server is the shared, company-scoped (and later
federation-scoped) counterpart to the local `.fukura/` directory.
Individual fukura clients — humans typing in shells, agents calling
MCP — produce [`Note`](../src/domain/models.rs) and
[`SolutionAttempt`](../src/domain/attempt.rs) records. A hub receives
those records from many clients, enforces privacy boundaries on
ingest, and lets consumers query the aggregate.

The hub does **not** replace the local repo. Notes are captured and
redacted locally; only what the privacy rules permit is ever
transmitted. Fukurahub is the read-and-share layer, not the system of
record.

This document specifies the wire API between client and hub. Server
implementation is out of scope — the spec is deliberately independent
so a company can run the reference server, a managed SaaS, or
anything EKP-compatible.

## 2. Transport

- **Protocol:** HTTP/1.1 or HTTP/2 over TLS.
- **Content type:** `application/json` for all request and response
  bodies unless otherwise noted.
- **Character set:** UTF-8.
- **Base URL:** configurable per client, e.g. `https://hub.example.com`.
- **API versioning:** every endpoint is prefixed with `/v1/`. v1 is
  the contract this document describes. Breaking changes require a
  new prefix (`/v2/`).

## 3. Authentication

- **Primary:** bearer tokens via `Authorization: Bearer <token>`.
  Tokens are opaque to the client; scopes and lifetimes are server
  policy.
- **Secondary (CI / agent):** static API keys over the same
  `Authorization: Bearer …` header. Servers MAY distinguish the two
  via claims in their own token format but clients do not care.
- **Anonymous reads:** a hub MAY expose read-only endpoints without a
  token for the `public` privacy tier. Write endpoints MUST always
  require authentication.

Rejected requests return `401 Unauthorized` with an empty body.

## 4. Privacy tiers

Every ingested record carries one of the tiers defined in
[`Privacy`](../src/domain/models.rs):

| Tier | Semantics |
| --- | --- |
| `private` | Client-local only. MUST NOT be transmitted; servers MUST reject uploads that carry this tier and respond `422 Unprocessable Entity`. |
| `org`     | Visible only to members of the uploader's organisation. Default tier for team use. |
| `public`  | Visible to every authenticated reader of the hub, and (if the server exposes anonymous reads) to the world. |

A future EKP version will add a `federated` tier for cross-org
fingerprint sharing. v0.1 servers MUST reject it.

## 5. Resources

### 5.1 Notes

A `Note` is the human-readable record; the wire shape is the
serialisation of [`NoteEnvelope`](../src/domain/models.rs) with its
embedded `ontology` field. Servers SHOULD accept both the v1 envelope
and future forward-compatible envelopes (ignoring unknown top-level
fields).

#### `POST /v1/notes`

Upload a single note.

Request body: a `NoteEnvelope` (see EKP §4.1 and
`src/domain/models.rs`). `privacy` MUST be `org` or `public`.

Response `201 Created`:

```json
{
  "object_id": "sha256:…",
  "url": "https://hub.example.com/v1/notes/sha256:…",
  "ontology": { "fingerprint": "sha256:…", "category": "cargo.compile.e0308" }
}
```

Errors:

- `400 Bad Request` — malformed envelope.
- `401 Unauthorized` — missing / invalid token.
- `403 Forbidden` — authenticated but not permitted to write under
  this privacy tier.
- `413 Payload Too Large` — see §9.
- `422 Unprocessable Entity` — privacy=`private`, or unknown/future
  privacy tier.

#### `GET /v1/notes/{object_id}`

Fetch a single note. `200 OK` returns the same envelope; `404 Not
Found` when the id is unknown or the caller cannot see the privacy
tier.

#### `GET /v1/notes`

Search notes. Query parameters:

- `q` (string, optional) — full-text query applied server-side.
- `fingerprint` (string, optional) — exact match on EKP fingerprint.
- `category` (string, optional) — exact match (e.g.
  `cargo.compile.e0308`) or hierarchical prefix with a trailing `.*`.
- `tag` (repeated) — zero or more tag filters, AND-combined.
- `privacy` (repeated) — one of `org`, `public`. Default is the union
  of everything the caller can see.
- `cursor` (opaque string) — pagination; see §7.
- `limit` (integer) — 1..=100, default 20.

Response `200 OK`:

```json
{
  "hits": [
    {
      "object_id": "sha256:…",
      "title": "cargo build: unresolved import",
      "category": "cargo.compile.e0432",
      "fingerprint": "sha256:…",
      "tags": ["cargo", "rust"],
      "updated_at": "2026-04-18T10:22:31Z",
      "summary": "…",
      "effectiveness": { "success": 12, "failure": 3, "total": 15, "success_rate": 0.8 }
    }
  ],
  "next_cursor": "opaque"
}
```

The `effectiveness` block is optional and only populated when the
server holds attempt data for the fingerprint (see §5.2).

### 5.2 Solution attempts

Attempts describe whether a fix actually worked. The shape is the
serialisation of [`SolutionAttempt`](../src/domain/attempt.rs).

#### `POST /v1/attempts`

Upload a batch of attempts. Batching is required because shell hooks
can emit high volumes: accepting one-at-a-time would waste
round-trips.

Request body:

```json
{ "attempts": [ /* SolutionAttempt, … */ ] }
```

Servers MAY cap the batch at a published size (§9).

Response `202 Accepted`:

```json
{ "accepted": 42, "rejected": 0, "errors": [] }
```

Partial acceptance is possible: per-item `errors` carry the index and
reason. Clients SHOULD retry only the rejected indices.

#### `GET /v1/attempts/stats`

Aggregate statistics.

Query parameters:

- `fingerprint` (optional) — restrict to one fingerprint; otherwise
  returns the top-N most-attempted fingerprints.
- `since` (ISO-8601 timestamp, optional) — only count attempts at or
  after this time.
- `limit` (integer, default 20, max 100) — number of fingerprints in
  the response.

Response `200 OK`:

```json
{
  "by_fingerprint": [
    {
      "fingerprint": "sha256:…",
      "category": "cargo.compile.e0432",
      "stats": { "success": 12, "failure": 3, "abandoned": 1, "total": 16, "success_rate": 0.75 }
    }
  ]
}
```

### 5.3 Health and metadata

#### `GET /v1/health`

Unauthenticated. Returns `200 OK` and a small JSON body:

```json
{ "status": "ok", "version": "0.1.0", "hub_id": "example-inc-hub" }
```

Clients use this for connectivity checks (§10) and to detect whether
the server advertises a specific `hub_id` they can pin.

#### `GET /v1/info`

Authenticated. Returns server policy the client needs to know:

```json
{
  "max_note_bytes": 262144,
  "max_attempts_per_batch": 500,
  "rate_limit_per_minute": 600,
  "retained_privacy_tiers": ["org", "public"],
  "server_time": "2026-04-18T10:22:31Z"
}
```

## 6. Idempotency

Uploads are naturally idempotent via the EKP fingerprint / object id.
Reposting the same note body MUST NOT create duplicate records;
servers return `200 OK` with the pre-existing `object_id` instead of
`201 Created`.

For `POST /v1/attempts`, every `SolutionAttempt` carries an
`attempt_id` (UUID). Servers MUST treat a repeated `attempt_id` as a
no-op and include it in the `accepted` count.

## 7. Pagination

Search and stats responses use opaque cursors. A response with a
`next_cursor` field indicates more results are available. Clients pass
the cursor back verbatim on the next request. Cursors are valid for at
least the server's `info.server_time + 1h` and MAY expire after.

Clients MUST NOT attempt to parse, decode, or construct cursors
themselves.

## 8. Rate limiting

Rate limits are expressed per token per minute. When exceeded the
server responds `429 Too Many Requests` with:

```
Retry-After: 30
X-RateLimit-Remaining: 0
```

Clients MUST honour `Retry-After` and SHOULD implement exponential
backoff on top (base 2s, cap 60s).

## 9. Size limits

- Single note body: default 256 KiB, advertised via
  `info.max_note_bytes`.
- Attempts batch: default 500 items, advertised via
  `info.max_attempts_per_batch`.
- Total request body: servers SHOULD accept at least 1 MiB and MUST
  reject anything above 8 MiB with `413 Payload Too Large`.

## 10. Client behaviour

Conforming clients MUST:

1. Apply local redaction before serialising anything destined for the
   hub (see `Redactor` in `src/domain/redaction.rs`).
2. Reject attempts to upload notes with `privacy = private` before
   they reach the wire.
3. Parse `info.max_note_bytes` / `info.max_attempts_per_batch` on
   first contact and honour them.
4. Retry `5xx` and `429` responses with exponential backoff.

Conforming clients SHOULD:

1. Cache `info` responses for up to one hour.
2. Report their own version in a `User-Agent` header of the form
   `fukura/<semver> (<producer>)`.
3. Include `Idempotency-Key` request headers on retries so servers
   can cleanly deduplicate in-flight uploads.

## 11. Errors

Error responses share a common body:

```json
{
  "error": {
    "code": "note_too_large",
    "message": "note body exceeds 262144 bytes",
    "retryable": false
  }
}
```

`code` values form a small, stable vocabulary; `message` is
human-readable and may change between releases. Clients MUST switch on
`code`, not `message`.

Initial code set:

- `invalid_envelope`
- `invalid_privacy`
- `note_too_large`
- `attempts_batch_too_large`
- `unauthorized`
- `forbidden`
- `not_found`
- `rate_limited`
- `server_unavailable`
- `internal_error`

## 12. Forward-compatibility

Servers MUST:

- Accept and ignore unknown top-level fields in note envelopes and
  attempts.
- Preserve unknown EKP ontology fields round-trip — they were added in
  a newer producer and other consumers may need them.

Clients MUST:

- Accept and ignore unknown fields in server responses.
- Treat unknown values of enum-like fields (severity, outcome) as
  `unknown` rather than erroring.

## 13. Non-goals for v0.1

- **Federation.** Cross-org sharing via k-anonymous fingerprint
  aggregation is a separate spec.
- **Streaming.** No WebSocket or SSE endpoints. Real-time
  notifications can be built on top later without breaking the
  request/response surface defined here.
- **Blob uploads.** Notes carry inline bodies; attachments wait for a
  future version with signed upload URLs.
- **Permissions model.** RBAC beyond "member of this org" is the
  hub's business. v0.1 only recognises `public` / `org`.

## 14. Change log

- **v0.1 (2026-04-18)** — initial draft. Defines the note upload
  surface, attempt batching, stats, health, info, pagination, and
  error model needed to run a useful hub.
