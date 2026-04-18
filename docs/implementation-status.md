# Implementation Status

Canonical cross-repo status matrix for every normative section of
`ekp-spec.md` and `fukurahub-api.md`. Any change that moves a row
between ✅ / 🚧 / ❌ / N/A MUST be landed in the same PR that changes
the underlying code, so this file stays current with HEAD.

## Legend

- **✅ Implemented in vX.Y** — shipped and covered by tests in the named version.
- **🚧 Planned for vX.Y** — spec is stable, implementation scheduled for that release.
- **❌ Not started** — no implementation and no concrete schedule.
- **N/A** — the spec section is not relevant to this repository.

Versions referenced: `fukura` (CLI crate) as of `v0.3.8`; `fukura-hub`
(`v0.1.0`); `fukura-site` (`v1.0.0`).

## EKP (`docs/ekp-spec.md`)

| Spec section | fukura CLI | fukura-hub | fukura-site |
|---|---|---|---|
| §4.1 Envelope (`schema`, `version`, `ontology`) | ✅ v0.3 | 🚧 v0.4 (P3 Slice 2 — `notes.ontology` JSONB column) | 🚧 v1.1 (/docs/ekp page, Task 2) |
| §4.2 Ontology required fields (adapter, category, fingerprint, occurred_at) | ✅ v0.3 | 🚧 v0.4 (P3 Slice 2) | 🚧 v1.1 |
| §4.2 Ontology optional fields (severity, signals, entities, tags, raw_excerpt, source_lineage) | ✅ v0.3 | 🚧 v0.4 (stored in JSONB, round-trip required) | 🚧 v1.1 |
| §5 Adapter framework (matches/parse contract) | ✅ v0.3 (cargo, git, kubernetes, generic) | N/A | 🚧 v1.1 (/docs/ekp page) |
| §5.3 Generic fallback adapter | ✅ v0.3 | N/A | 🚧 v1.1 |
| §6 Fingerprinting (BLAKE3 prefix, normalised signature, no variable values) | ✅ v0.3 | 🚧 v0.4 (P3 Slice 2 — indexed column) | 🚧 v1.1 |
| §7 Redaction (producer-side MUST) | ✅ v0.3 (`src/domain/redaction.rs`) | N/A (server never sees raw) | 🚧 v1.1 |
| §8 Entity vocabulary (starter set) | ✅ v0.3 | N/A | 🚧 v1.1 |
| §9 Effectiveness attempts (informative — concrete schema in hub API §5.2) | ✅ v0.3 (`AttemptStore`, `SolutionAttempt`) | 🚧 v0.4 (P3 Slice 1 — attempts table) | 🚧 v1.1 (/docs/effectiveness page) |
| §10 Version negotiation | ✅ v0.3 | 🚧 v0.4 (forward-compat round-trip for unknown ontology fields) | 🚧 v1.1 |
| §11 Conformance | ✅ v0.3 (producer) | 🚧 v0.4 (consumer) | N/A |

## Hub API (`docs/fukurahub-api.md`)

| Spec section | fukura CLI (client) | fukura-hub (server) | fukura-site (docs) |
|---|---|---|---|
| §2 Transport (HTTPS, JSON, UTF-8, `/v1/` prefix) | ✅ v0.3 (`HttpHubClient`) | 🚧 v0.4 (P3 Slice 3 — currently serves `/api/` only) | 🚧 v1.1 (/docs/hub-api) |
| §3 Authentication (`Authorization: Bearer`) | ✅ v0.3 | ✅ v0.1 (JWT + argon2 + OAuth; org scoping via claim 🚧 v0.4 Slice 3) | 🚧 v1.1 |
| §4 Privacy tiers (`private`/`org`/`public`) | ✅ v0.3 | 🚧 v0.4 (enum is `team`; P3 Slice 4 renames `team`→`org` in 3 stages) | 🚧 v1.1 |
| §5.1 `POST /v1/notes` (envelope in, `{object_id,url,ontology}` out) | ✅ v0.3 | 🚧 v0.4 (P3 Slice 3 — current `/api/notes` takes `CreateNoteRequest`) | 🚧 v1.1 |
| §5.1 `GET /v1/notes/{object_id}` | ✅ v0.3 | 🚧 v0.4 (P3 Slice 3 — current route is `/api/notes/:uuid`) | 🚧 v1.1 |
| §5.1 `GET /v1/notes/{object_id}` → **410 Gone** on deleted (alignment D4a) | ✅ v0.3 (tolerates) | ❌ not scheduled — rides with Slice 3 | 🚧 v1.1 |
| §5.1 `GET /v1/notes` search (q / fingerprint / category / tag / privacy / cursor / limit) | ✅ v0.3 | 🚧 v0.4 (P3 Slice 3 — current `/api/notes` is offset-paginated and has no fingerprint/category filters) | 🚧 v1.1 |
| §5.2 `POST /v1/attempts` (batch, partial accept) | ✅ v0.3 | 🚧 v0.4 (P3 Slice 1 — attempts table) | 🚧 v1.1 (/docs/effectiveness) |
| §5.2 `GET /v1/attempts/stats` | ✅ v0.3 | 🚧 v0.4 (P3 Slice 1) | 🚧 v1.1 |
| §5.3 `GET /v1/health` (unauth, `{status,version,hub_id}`) | ✅ v0.3 | 🚧 v0.4 (current route is `/health` and returns `{status,service,version}`) | 🚧 v1.1 |
| §5.3 `GET /v1/info` (authed policy advertisement) | ✅ v0.3 (1h cache added in v0.4 Task 3) | 🚧 v0.4 (P3 Slice 3) | 🚧 v1.1 |
| §6 Idempotency (repost body → 200 + existing `object_id`) | ✅ v0.4 (`Idempotency-Key` header on every POST) | 🚧 v0.4 (P3 Slice 3 — UPSERT on `object_id`) | N/A |
| §6 Idempotency (attempt_id dedup) | ✅ v0.3 | 🚧 v0.4 (P3 Slice 1) | N/A |
| §7 Pagination (opaque cursors, no client parsing) | ✅ v0.3 | 🚧 v0.4 (currently offset) | 🚧 v1.1 |
| §8 Rate limiting (429 + `Retry-After`, client honors + backoff) | ✅ v0.4 (Task 3) | 🚧 v0.4 (P3 Slice 3) | 🚧 v1.1 |
| §9 Size limits (256 KiB / 500 / 8 MiB → 413) | ✅ v0.4 (Task 3) (client-side check) | 🚧 v0.4 (P3 Slice 3) | 🚧 v1.1 |
| §10.1 Client MUST: local redaction before send | ✅ v0.3 | N/A | 🚧 v1.1 |
| §10.2 Client MUST: reject `private` pre-network | ✅ v0.4 (Task 3) | N/A | N/A |
| §10.3 Client MUST: parse + honor `info` limits | ✅ v0.4 (Task 3) (1h cache) | N/A | N/A |
| §10.4 Client MUST: retry 5xx/429 with backoff | ✅ v0.4 (Task 3) | N/A | N/A |
| §10.5 Client SHOULD: `Idempotency-Key` on retries | ✅ v0.4 (Task 3) | 🚧 v0.4 (server dedup, Slice 3) | N/A |
| §11 Error body (`{code,message,retryable}`) | ✅ v0.3 (parses both shapes today) | 🚧 v0.4 (P3 Slice 3 — currently `{"error":"<string>"}`) | 🚧 v1.1 |
| §12 Forward compatibility (ignore unknown fields, round-trip unknown ontology) | ✅ v0.3 | 🚧 v0.4 (P3 Slice 2 — JSONB ontology) | N/A |

## MCP (Claude Code / Cursor integration)

| Capability | fukura CLI | fukura-hub | fukura-site |
|---|---|---|---|
| `fuku mcp` JSON-RPC 2.0 stdio server | ✅ v0.3 | N/A | 🚧 v1.1 (/docs/mcp) |
| `fukura_classify`, `fukura_search`, `fukura_record`, `fukura_preflight` tools | ✅ v0.3 | N/A | 🚧 v1.1 |
| `fukura_record_attempt` tool | ✅ v0.3 | N/A | 🚧 v1.1 |
| `fuku claude-code register` (idempotent merge-patch of `~/.claude.json` / `.mcp.json`) | ✅ v0.3 | N/A | 🚧 v1.1 |

## Contract testing (alignment doc §4)

| Capability | fukura CLI | fukura-hub CI |
|---|---|---|
| `tests/hub_http_client.rs` runs against in-process mock (default) | ✅ v0.3 | N/A |
| Same test file runs against real hub via `HUB_BASE_URL` env | ✅ v0.4 (Task 1) | 🚧 v0.4 (Task 4) |
| Contract test job in hub CI blocks merges on spec violations | N/A | 🚧 v0.4 (Task 4 — initial snapshot mode, then gate) |

## Change policy

When you edit fukura code that implements a spec section, **update
the matching row in this file in the same commit**. Reviewers should
reject a PR that bumps behavior without bumping status. Cross-repo
changes update the matching row in each repo's PR description.
