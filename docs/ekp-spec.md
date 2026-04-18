# Error Knowledge Protocol (EKP) — v0.1 (Draft)

**Status:** Draft / Request for Comments
**Editor:** Fukura contributors
**License:** CC-BY 4.0

## 1. Motivation

Every engineering team re-solves the same errors. Logs get lost, Slack threads
fade, and the knowledge of "what actually fixed this" lives only in a few
people's heads. Tools exist for runtime error tracking (Sentry, Datadog), for
general knowledge (Notion, Glean), and for code suggestions (Copilot, Cursor),
but nothing standardises the representation of a **developer-facing error and
its fix** in a way that is portable across tools, companies, and agents.

This document specifies EKP, a wire-level schema for describing an error
occurrence and a solution attempt such that the data can be:

- captured from heterogeneous sources (shells, CI, CLIs, agents),
- deduplicated across machines and users,
- queried by both humans and LLM agents,
- shared across organisations without leaking secrets.

EKP is to error-and-fix data what LSP is to editor-and-language integrations:
a shared protocol that lets independent components interoperate.

## 2. Design goals

1. **Fingerprint stability.** The same logical error must produce the same
   fingerprint regardless of ephemeral values (paths, UUIDs, timestamps).
2. **Tool agnosticism.** The schema must not assume a specific shell, CLI, or
   language.
3. **Privacy by default.** All secret-bearing fields must be redactable at the
   source; the schema distinguishes redacted from raw values explicitly.
4. **Extensibility without breakage.** New adapters can add fields without
   forcing older consumers to change.
5. **Small surface.** A conforming producer must emit fewer than a dozen
   required fields.

## 3. Terminology

| Term | Meaning |
| --- | --- |
| **Producer** | Anything that emits EKP documents (shell hook, daemon, agent) |
| **Consumer** | Anything that reads EKP documents (search, UI, LLM agent) |
| **Adapter** | A producer component specialised for a tool, CLI, or environment |
| **Fingerprint** | A stable identifier for "the same error" across invocations |
| **Ontology** | The structured EKP representation of a single error occurrence |
| **Entity** | A named resource referenced in an error (cluster, pod, repo, etc.) |

## 4. Data model

### 4.1 Envelope

Every EKP document is wrapped in an envelope:

```json
{
  "schema": "fuku.ekp",
  "version": 1,
  "ontology": { ... }
}
```

- `schema` MUST be the literal string `fuku.ekp`.
- `version` is an integer. This document describes version `1`.
- Unknown top-level fields MUST be ignored by consumers.

### 4.2 Ontology

```json
{
  "adapter": "kubernetes",
  "adapter_version": "0.1.0",
  "category": "kubernetes.image_pull",
  "severity": "blocking",
  "fingerprint": "blake3:7f9c…",
  "occurred_at": "2026-04-18T10:22:31Z",
  "signals": { ... },
  "entities": [ ... ],
  "tags": ["kubernetes", "image-pull"],
  "raw_excerpt": "...(redacted)...",
  "source_lineage": { ... }
}
```

#### Required fields

- `adapter` *(string)* — ID of the adapter that produced the document. See §5.
- `category` *(string)* — Dotted hierarchical identifier, e.g.
  `kubernetes.image_pull`, `cargo.compile.e0308`, `git.merge.conflict`,
  `generic.unknown`.
- `fingerprint` *(string)* — Opaque stable identifier, prefixed with the hash
  algorithm (e.g. `blake3:…`). See §6.
- `occurred_at` *(RFC 3339 timestamp)* — Wall-clock time at the producer.

#### Optional fields

- `adapter_version` *(semver)* — Useful when a fingerprint depends on adapter
  logic that may change between versions.
- `severity` *(enum)* — One of `blocking`, `warning`, `info`, `unknown`.
  Default is `unknown`.
- `signals` — Structured low-level evidence used to derive the category and
  fingerprint. See §4.3.
- `entities` — Array of named resources referenced by the error. See §4.4.
- `tags` — Flat array of human-oriented tags. Adapters SHOULD prefer
  `category` for machine logic and use tags for UI/search affinity.
- `raw_excerpt` — A length-limited (<= 2048 chars) redacted snippet of the
  original error output, for human review. MUST have all secrets removed.
- `source_lineage` — Where this ontology came from. See §4.5.

### 4.3 Signals

```json
{
  "exit_code": 1,
  "error_code": "ImagePullBackOff",
  "command_head": "kubectl apply",
  "stderr_pattern": "Failed to pull image ***",
  "stdout_bytes": 0,
  "stderr_bytes": 421,
  "duration_ms": 1820
}
```

All fields are optional. `stderr_pattern` MUST already be redacted and
normalised (variable parts replaced with `***`).

### 4.4 Entities

```json
[
  { "type": "cluster", "value": "prod-us-east", "redacted": false },
  { "type": "namespace", "value": "payments", "redacted": false },
  { "type": "image", "value": "***", "redacted": true }
]
```

- `type` SHOULD be drawn from a well-known vocabulary when possible. A
  starter set is defined in §8. Unknown types are permitted.
- `value` MUST NOT contain secrets when `redacted` is `false`.

### 4.5 Source lineage

```json
{
  "producer": "fukura",
  "producer_version": "0.4.0",
  "host_os": "linux",
  "host_arch": "x86_64",
  "shell": "zsh",
  "agent": { "kind": "claude-code", "version": "..." }
}
```

Every field is optional. `agent`, when present, records that the error was
emitted by an autonomous agent ("banging on tools") rather than a human.
This distinction matters for effectiveness feedback (§9).

## 5. Adapters

An **Adapter** is a component that parses raw invocation context (command,
exit code, stderr, working directory, environment) and emits an EKP
ontology. Adapters are identified by a stable string ID.

### 5.1 Adapter contract

A conforming Adapter MUST:

1. Expose a stable ID (lowercase, `[a-z0-9_-]+`) and a semver version.
2. Implement a `matches(context) -> bool` decision function that is
   side-effect free.
3. Implement a `parse(context) -> Ontology` function that returns a valid
   EKP document when `matches` returned `true`.
4. Never mutate the input context.
5. Never emit unredacted secrets in `raw_excerpt` or `signals.stderr_pattern`.

An Adapter SHOULD:

- Declare its priority so the registry can disambiguate overlapping matches
  (higher priority wins).
- Declare a set of redaction patterns specific to the environments it
  targets.

### 5.2 Discovery

v0.1 supports only in-process adapters statically registered at build time.
Future versions will specify out-of-process adapters (via stdio / MCP) and a
manifest format for third-party packs.

### 5.3 Fallback adapter

Producers MUST ship a **generic** fallback adapter that always matches and
produces an ontology with `category = "generic.unknown"`. This guarantees
that every captured error yields an EKP document.

## 6. Fingerprinting

The fingerprint identifies "the same logical error". It is computed as
follows:

1. Adapters produce a **normalised signature string** composed of:
   - `adapter` ID,
   - `category`,
   - stable signals (error code, command head, normalised stderr pattern),
   - entity *types* (not values).
2. The signature is hashed with BLAKE3 (or SHA-256 as a fallback). The hex
   digest is prefixed with the algorithm name, e.g. `blake3:7f9c…`.
3. Adapters MUST NOT include variable values (paths, UUIDs, timestamps,
   hostnames) in the signature.
4. Adapters MAY include the adapter version in the signature when a
   fingerprint change is intended across adapter releases.

Producers MAY attach additional finer-grained fingerprints under
`signals.sub_fingerprints` for clustering experiments.

## 7. Redaction

- Producers MUST apply redaction before serialising an EKP document.
- The default redaction set includes credentials, tokens, private keys,
  database URLs, emails, and IPs.
- Adapters MAY contribute environment-specific patterns (e.g. internal
  hostnames, internal ticket IDs).
- When a field is redacted, its sibling `redacted` flag (when present)
  MUST be set to `true`.

## 8. Entity vocabulary (starter set)

Adapters SHOULD prefer these types when applicable. Unknown types are
permitted but reduce cross-adapter interoperability.

| Type | Example |
| --- | --- |
| `cluster` | `prod-us-east` |
| `namespace` | `payments` |
| `pod` | redacted |
| `image` | `registry.example/app:1.2.3` |
| `service` | `checkout-api` |
| `repo` | `boostbit-inc/fukura` |
| `branch` | `main` |
| `commit` | `abc1234` |
| `package` | `tokio` |
| `module` | `fukura::adapter` |
| `file` | redacted path |
| `registry` | `gcr.io`, internal registry redacted |
| `user` | redacted |

## 9. Effectiveness feedback (informative)

A separate EKP document type, `fuku.ekp.attempt`, will describe a solution
attempt and its outcome. The v0.1 document reserves the `attempt` name but
does not yet define its schema. The intent:

- Producers (shell hooks, agents) record the commands that followed an
  error and whether the next successful command resolved it.
- This yields a measured success rate per solution, per fingerprint.
- Version 2 of EKP will standardise this document.

## 10. Versioning and compatibility

- Consumers MUST accept documents whose `version` is less than or equal to
  the version they were built for.
- Producers SHOULD emit the highest version supported by all downstream
  consumers in their deployment.
- Field additions in minor versions MUST be optional; field removals or
  semantic changes require a major version bump.

## 11. Conformance

A producer is **EKP v1 conformant** if it:

- emits valid envelopes,
- provides at least a generic fallback adapter,
- computes fingerprints per §6,
- applies redaction per §7.

A consumer is **EKP v1 conformant** if it:

- accepts valid envelopes without error,
- treats unknown optional fields as permissible,
- treats unknown enum values as `unknown`.

## 12. Open questions

- Should `fingerprint` include adapter version by default, or be independent
  so fingerprints remain stable across adapter upgrades? (Currently optional
  per adapter; likely to be pinned in v2.)
- How should cross-adapter fingerprint collisions be handled when two
  adapters produce the same fingerprint for different categories?
- What is the minimum viable `attempt` document (§9) that still enables a
  useful effectiveness loop?
- Federation: how should `fingerprint` aggregation across organisations
  preserve k-anonymity? A separate federation spec will address this.

## 13. Change log

- v0.1 (2026-04-18): Initial draft.
