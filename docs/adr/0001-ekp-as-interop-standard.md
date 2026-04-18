# ADR 0001: EKP as the interoperability standard

Date: 2026-04-18
Status: Accepted

## Context

Several adjacent tools capture developer-facing error data — Sentry and
Datadog for runtime production errors, Warp and Pieces.app for
dev-time shell history, Stack Overflow Teams and Glean for manual
knowledge capture, and every major agent (Claude Code, Cursor, Devin)
for their own internal memory. None share a schema. A fix that worked
in Cursor is invisible to Claude Code and vice versa; a corpus built
on Warp cannot be queried from a shell hook.

Fukura targets the wedge none of them occupy: **CLI-native, schema-
standardised, agent-aware, effectiveness-measured, local-first**.

The schema half of that positioning only pays off if the schema is
public, stable, and adoptable by other vendors. Otherwise fukura is
just another silo.

## Decision

Publish the Error Knowledge Protocol (`docs/ekp-spec.md`) under a
permissive licence (CC-BY 4.0) as an open standard, framed as an RFC
rather than fukura-internal documentation. Adapter framework, wire
format, fingerprint rules, redaction requirements, and conformance
criteria are all in the spec.

Fukura's own implementation is Apache-2.0 and serves as the reference
producer. Third parties are expected to adopt the same wire format;
when that happens, fukura becomes the interop layer rather than the
only layer.

## Consequences

- Spec changes are public by default. Breaking changes require a
  version bump and a migration note, not a silent implementation
  change.
- Competitors can implement EKP without touching fukura code; this is
  intentional. The moat is the data corpus + effectiveness
  measurement, not schema lock-in.
- Every fukura feature that emits or consumes an ontology must
  reference the spec section it implements. If the feature needs a
  field the spec lacks, the spec changes first.
- Documentation responsibility: the canonical `ekp-spec.md` lives in
  this repo; rendered versions on `fukura-site` are summaries, never
  divergent.
