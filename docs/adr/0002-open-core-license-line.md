# ADR 0002: Open-core line between CLI and hub

Date: 2026-04-18
Status: Accepted

## Context

Fukura is positioned as two complementary components:

- A **local CLI** that runs on a developer's machine and captures
  error/fix events, redacts them, and builds a content-addressable
  local store. Adoption depends on broad distribution — any friction
  (licence worries, paid trial) cuts off the pipeline at its source.
- A **hub** that aggregates captured data across many machines and
  makes it queryable for an organisation. Value scales with the
  number of users connected; this is where commercial logic lives.

Both cannot be permissively licensed without giving away the
commercial surface; both cannot be commercial-only without strangling
adoption.

## Decision

Draw the open-core line at the CLI / hub boundary:

- **Apache-2.0 (public)**: the CLI crate, EKP spec, adapter SDK, MCP
  server, client library, documentation. Everything in
  `boostbit-inc/fukura` and `boostbit-inc/fukura-site`.
- **Source-available or commercial**: the hub server (fukurahub),
  enterprise features (SSO, RBAC, federation, audit). Everything in
  `boostbit-inc/fukura-hub` — currently a private repo.

The client library speaks a published HTTP spec
(`docs/fukurahub-api.md`, also Apache-2.0). Any EKP-compatible server
can be substituted: the reference implementation, a managed SaaS, or
a third-party build. That guarantees the CLI never becomes a hostage
to our commercial hub.

## Consequences

- External contributions to the hub require a CLA (not yet in place —
  tracked in `handoff.md`).
- Features that genuinely serve everyone (redaction rules, adapters,
  MCP wiring, client resilience) must live in the CLI half, even when
  it would be simpler to implement them server-side.
- Features that only make sense at aggregate scale (federation, RBAC,
  dashboards) live in the hub half. They are not copied into the CLI
  as a subset.
- Documentation responsibility: spec docs stay in the public repo
  (`boostbit-inc/fukura/docs/`); hub implementation roadmaps live in
  the private hub repo (`boostbit-inc/fukura-hub/docs/`).
