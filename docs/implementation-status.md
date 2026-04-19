# Implementation Status

Canonical cross-repo status matrix for every normative section of
`ekp-spec.md` and `fukurahub-api.md`, plus the product surfaces built
on top. Any change that moves a row between ✅ / 🚧 / ❌ / N/A MUST
be landed in the same PR that changes the underlying code, so this
file stays current with HEAD.

## Legend

- **✅ Implemented in vX.Y** — shipped and covered by tests in the named version.
- **🚧 Planned for vX.Y** — spec is stable, implementation scheduled for that release.
- **❌ Not started** — no implementation and no concrete schedule.
- **N/A** — the spec section is not relevant to this repository.

Versions referenced: `fukura` (CLI crate) as of `v0.3.8`; `fukura-hub`
(`v0.1.0`); `fukura-site` (`v1.x`).

## EKP (`docs/ekp-spec.md`)

| Spec section | fukura CLI | fukura-hub | fukura-site |
|---|---|---|---|
| §4.1 Envelope (`schema`, `version`, `ontology`) | ✅ v0.3 | ✅ v0.1 (`notes.ontology` JSONB round-trips) | ✅ v1.x (`/docs/ekp`) |
| §4.2 Ontology required fields (adapter, category, fingerprint, occurred_at) | ✅ v0.3 | ✅ v0.1 | ✅ v1.x |
| §4.2 Ontology optional fields (severity, signals, entities, tags, raw_excerpt, source_lineage) | ✅ v0.3 | ✅ v0.1 (JSONB preserves unknown fields) | ✅ v1.x |
| §5 Adapter framework (matches/parse contract) | ✅ v0.3 (cargo, git, kubernetes, **python, node, docker, terraform, generic**) | N/A | ✅ v1.x |
| §5.3 Generic fallback adapter | ✅ v0.3 | N/A | ✅ v1.x |
| §6 Fingerprinting (BLAKE3 prefix, normalised signature, no variable values) | ✅ v0.3 | ✅ v0.1 (indexed `notes.fingerprint`) | ✅ v1.x |
| §7 Redaction (producer-side MUST) | ✅ v0.3 (`src/domain/redaction.rs`) | N/A (server never sees raw) | ✅ v1.x (`/security`) |
| §8 Entity vocabulary (starter set) | ✅ v0.3 | N/A | ✅ v1.x |
| §9 Effectiveness attempts (informative — concrete schema in hub API §5.2) | ✅ v0.3 (`AttemptStore`, `SolutionAttempt`) | ✅ v0.1 (P3 Slice 1) | ✅ v1.x (`/docs/effectiveness`) |
| §10 Version negotiation | ✅ v0.3 | ✅ v0.1 (ontology JSONB round-trips unknown fields) | ✅ v1.x |
| §11 Conformance | ✅ v0.3 (producer) | ✅ v0.1 (consumer) | N/A |

## Hub API (`docs/fukurahub-api.md`)

| Spec section | fukura CLI (client) | fukura-hub (server) | fukura-site (docs) |
|---|---|---|---|
| §2 Transport (HTTPS, JSON, UTF-8, `/v1/` prefix) | ✅ v0.3 (`HttpHubClient`) | ✅ v0.1 (P3 Slice 3; `/api/*` coexists per ADR 0003) | ✅ v1.x (`/docs/hub-api`) |
| §3 Authentication (`Authorization: Bearer`) | ✅ v0.3 | ✅ v0.1 (JWT + argon2 + OAuth + OIDC; org_id claim scoping via `/api/auth/switch-context`, ADR 0004) | ✅ v1.x |
| §4 Privacy tiers (`private`/`org`/`public`) | ✅ v0.3 | ✅ v0.1 (P3 Slice 4 `team`→`org` rename; legacy `private` purged per ADR 0009) | ✅ v1.x |
| §5.1 `POST /v1/notes` (envelope in, `{object_id,url,ontology}` out) | ✅ v0.3 | ✅ v0.1 | ✅ v1.x |
| §5.1 `GET /v1/notes/{object_id}` | ✅ v0.3 | ✅ v0.1 | ✅ v1.x |
| §5.1 `GET /v1/notes/{object_id}` → **410 Gone** on deleted (alignment D4a) | ✅ v0.3 (tolerates) | ✅ v0.1 (checks `deleted_at`) | ✅ v1.x |
| §5.1 `GET /v1/notes` search (q / fingerprint / category / tag / privacy / cursor / limit) | ✅ v0.3 | ✅ v0.1 (cursor pagination, `category.*` prefix) | ✅ v1.x |
| §5.2 `POST /v1/attempts` (batch, partial accept) | ✅ v0.3 | ✅ v0.1 | ✅ v1.x (`/docs/effectiveness`) |
| §5.2 `GET /v1/attempts/stats` | ✅ v0.3 | ✅ v0.1 | ✅ v1.x |
| §5.3 `GET /v1/health` (unauth, `{status,version,hub_id}`) | ✅ v0.3 | ✅ v0.1 (legacy `/health` kept) | ✅ v1.x |
| §5.3 `GET /v1/info` (authed policy advertisement) | ✅ v0.3 (1h cache) | ✅ v0.1 | ✅ v1.x |
| §6 Idempotency (repost body → 200 + existing `object_id`) | ✅ v0.4 (`Idempotency-Key` header on every POST) | ✅ v0.1 (content-addressable sha256 object_id, UNIQUE constraint) | N/A |
| §6 Idempotency (attempt_id dedup) | ✅ v0.3 | ✅ v0.1 (UNIQUE attempt_id + ON CONFLICT DO NOTHING) | N/A |
| §7 Pagination (opaque cursors, no client parsing) | ✅ v0.3 | ✅ v0.1 (base64 offset cursor) | ✅ v1.x |
| §8 Rate limiting (429 + `Retry-After`, client honors + backoff) | ✅ v0.4 | ✅ v0.1 (per-user token bucket at 600 req/min, 429 + Retry-After on exceed) | ✅ v1.x |
| §9 Size limits (256 KiB / 500 / 8 MiB → 413) | ✅ v0.4 (client-side check) | ✅ v0.1 (POST /v1/notes → 413 over 256 KiB; batch cap on attempts) | ✅ v1.x |
| §10.1 Client MUST: local redaction before send | ✅ v0.3 | N/A | ✅ v1.x |
| §10.2 Client MUST: reject `private` pre-network | ✅ v0.4 | N/A | N/A |
| §10.3 Client MUST: parse + honor `info` limits | ✅ v0.4 (1h cache) | N/A | N/A |
| §10.4 Client MUST: retry 5xx/429 with backoff | ✅ v0.4 | N/A | N/A |
| §10.5 Client SHOULD: `Idempotency-Key` on retries | ✅ v0.4 | ✅ v0.1 (server dedups via content-addressable object_id + attempt_id) | N/A |
| §11 Error body (`{code,message,retryable}`) | ✅ v0.3 (parses both shapes) | ✅ v0.1 on `/v1/*` via `api::errors::v1_error`; `/api/*` keeps plain-string for backcompat | ✅ v1.x |
| §12 Forward compatibility (ignore unknown fields, round-trip unknown ontology) | ✅ v0.3 | ✅ v0.1 (serde ignores unknowns, ontology + extras stored as JSONB and round-tripped) | N/A |
| Server-side `privacy=private` rejection (spec §4 MUST) | N/A | ✅ v0.1 (422 invalid_privacy in `/v1/notes`; legacy rows purged per ADR 0009) | N/A |
| `JWT_SECRET` refuse-to-start guard (no weak-default boot) | N/A | ✅ v0.1 (config.rs rejects empty, "dev-secret-*", placeholder, or <32 chars) | N/A |

## MCP (Claude Code / Cursor / Continue.dev / Zed integration)

| Capability | fukura CLI | fukura-hub | fukura-site |
|---|---|---|---|
| `fukura mcp` JSON-RPC 2.0 stdio server | ✅ v0.3 | N/A | ✅ v1.x (`/docs/mcp`) |
| `fukura_classify`, `fukura_search`, `fukura_record`, `fukura_preflight`, `fukura_record_attempt` tools | ✅ v0.3 | N/A | ✅ v1.x |
| `fukura claude-code register` (idempotent merge-patch of `~/.claude.json` / `.mcp.json`) | ✅ v0.3 | N/A | ✅ v1.x |
| `fukura cursor register` | 🚧 task 50 | N/A | ✅ v1.x (manual config documented) |
| Continue.dev + Zed configurations documented | N/A | N/A | ✅ v1.x |

## Spec extensions (beyond MUSTs)

Features the spec lists as non-goals or explicit follow-ups that are
shipping anyway as hub-specific extensions.

| Capability | fukura-hub | fukura-site | Notes |
|---|---|---|---|
| Note-body forward compat (round-trip `links` / `meta` / `solutions` unchanged) | ✅ v0.1 (JSONB `extras` column) | N/A | Beyond spec §12 MUST — spec only requires "ignore"; we preserve. |
| SSE real-time attempt feed `GET /v1/stream/attempts` | ✅ v0.1 (hub extension; documented as non-portable) | ✅ v1.x (surfaced in `/effectiveness` LiveFeed) | Spec §13 listed this as "non-goal, can be built on top later". In-process broadcast, authenticated. |
| Agent-kind split `GET /v1/attempts/stats/by-agent` | ✅ v0.1 (hub extension) | ✅ v1.x (HumanVsAgentSplit widget) | Unblocks the human-vs-agent widget on the effectiveness dashboard. |
| Federation / k-anonymity | 🚧 draft spec (`docs/ekp-federation.md`) | 🚧 v1.x | v0.1 draft with k=5 default, per-window salted hub-id hashes, Ed25519-signed contributions. Implementation pending spec review. |

## Product surfaces (user-facing screens and commands)

| Surface | Status | Notes |
|---|---|---|
| Hub `/effectiveness` dashboard (post-login landing: \$ wasted hero tile, pain-rank, human-vs-agent split, recurring-pattern list, coverage donut, live SSE feed) | ✅ v0.1 | |
| Hub `/digest` printable weekly report (CFO-forwardable artifact) | ✅ v0.1 | |
| Hub `/organizations` (create orgs, list members, invite by email) | ✅ v0.1 | Backed by existing `/api/organizations` CRUD. |
| Hub `/audit-log` viewer + CSV export | ✅ v0.1 | Reads `/api/audit-log`; server-scoped (admins see org, members see self). |
| Hub `ContextSwitcher` in Navbar (personal / org context toggle via JWT `org_id` claim) | ✅ v0.1 | |
| Hub `/notes`, `/notes/new`, `/notes/[id]/edit`, `/search`, `/profile`, `/dashboard` (legacy analytics) | ✅ v0.1 | |
| Hub `/beta` signup callback | 🚧 planned | `/api/beta-signups` exists; no on-hub confirmation page yet. |
| Hub `/settings` (token rotation, notification prefs) | 🚧 task 51 | |
| Hub `/billing` (upgrade path Personal → Team) | ❌ not started | Pricing copy lives on the marketing site; no in-product upgrade flow. |
| `fukura init --dry-run / --no-daemon / --no-hooks` | ✅ v0.4 | Preview mode added after first-time-user review. |
| `fukura export` (NDJSON of notes + attempts) | ✅ v0.4 | Proves the "no vendor lock-in" claim. |
| `fukura dashboard` (localhost web UI, hub not required) | ✅ v0.4 | The solo-dev tier. |
| `fukura hub seed-demo` (populate realistic demo data) | ✅ v0.4 | For screenshots and first-run demos. |
| `fukura doctor` (self-check command) | 🚧 task 49 | |
| Site home / problem / how-it-works / metric / CTA | ✅ v1.x | Rewritten with Claude Code brand named in hero. |
| Site `/pricing` (Free / Personal $12 / Team $12k yr / Enterprise from $60k) | ✅ v1.x | Four tiers after solo-dev feedback. |
| Site `/beta` (managed-hub waitlist form, posts to `/api/beta-signups`) | ✅ v1.x | |
| Site `/compare` (7 competitor paragraphs) | ✅ v1.x | |
| Site `/why-fukura` (positioning thesis per ADR 0008) | ✅ v1.x | |
| Site `/docs/quickstart`, `/docs/deploy-hub`, `/docs/cli-reference`, `/docs/ekp`, `/docs/hub-api`, `/docs/mcp`, `/docs/effectiveness` | ✅ v1.x | |
| Site `/security` | ✅ v1.x | |
| Site `/install.sh` (honest installer with `FUKURA_INSTALL_DIR` override, SHA256, APT fallback) | ✅ v1.x | Already existed; sudo-free path added. |

## Contract testing (alignment doc §4)

| Capability | fukura CLI | fukura-hub CI |
|---|---|---|
| `tests/hub_http_client.rs` runs against in-process mock (default) | ✅ v0.3 | N/A |
| Same test file runs against real hub via `HUB_BASE_URL` env | ✅ v0.4 | ✅ v0.1 (workflow wires HUB_BASE_URL + per-slice HUB_SLICE_N markers) |
| Contract test job in hub CI blocks merges on spec violations | N/A | ✅ (gating; per-test skips via `HUB_SLICE_<N>` env vars as slices land) |

## Change policy

When you edit fukura code that implements a spec section, **update
the matching row in this file in the same commit**. Reviewers should
reject a PR that bumps behavior without bumping status. Cross-repo
changes update the matching row in each repo's PR description.
