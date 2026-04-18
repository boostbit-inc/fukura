# Fukura governance

Operating rules for making changes that touch more than one repository
in the fukura family (`fukura`, `fukura-hub`, `fukura-site`).

## Source of truth

The normative specifications live in this repository under
`fukura/docs/`:

- `ekp-spec.md` — Error Knowledge Protocol v1.
- `fukurahub-api.md` — Hub HTTP API v1.

Any behaviour of the hub server, the CLI, or the public site that is
covered by these documents MUST follow them. When an implementation
needs to diverge, the spec is updated first (with an ADR — see below),
then the implementation.

The current cross-repo state is tracked in
`fukura/docs/implementation-status.md`. A PR that moves an
implementation row MUST update that file in the same commit.

## ADRs

Architectural decisions — anything that constrains future code beyond
a single change — are recorded as Architecture Decision Records under
`fukura/docs/adr/NNNN-<slug>.md`.

Format: one page, three sections — **Context**, **Decision**,
**Consequences**. Status is one of `Proposed`, `Accepted`, `Superseded
by ADR-MMMM`, or `Rejected`. Numbering is strictly sequential;
filling a gap is not allowed (superseding is — link it in the header).

When an ADR is accepted it may obsolete part of a spec. In that case
the spec PR lands first and references the ADR by number.

## Cross-repo changes

A change that touches two or more of {`fukura`, `fukura-hub`,
`fukura-site`}:

1. Update `docs/implementation-status.md` in `fukura` first, in its
   own PR, so the intended target state is public before the work
   starts.
2. Each sibling PR links the others in its description
   (`Related: boostbit-inc/fukura#NNN`, etc.).
3. Tags are cut together — see `CHANGELOG.md` for the coordinated
   release convention.

Contract tests (`fukura/tests/hub_http_client.rs`, run against a real
hub via `HUB_BASE_URL` from `fukura-hub`'s CI) are the final arbiter
of whether the server matches the spec. A merge that breaks those
tests in hub's CI is reverted, not excused.

## Scope that bypasses governance

Two narrow categories do **not** require an ADR or a
`implementation-status.md` bump:

- Pure cleanup: typo fixes, whitespace, non-semantic refactors.
- Non-normative additions: internal tests, benchmarks, build tooling,
  CI plumbing that doesn't change user-facing behaviour.

If a PR description cannot honestly claim either of the above, the
governance steps apply.
