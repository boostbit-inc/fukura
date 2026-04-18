# Claude Code rules for fukura (CLI)

This file is auto-loaded by Claude Code at session start. It applies
to the public CLI repo. Companion files live in `fukura-hub/CLAUDE.md`
and `fukura-site/CLAUDE.md`.

## Source of truth

- `docs/ekp-spec.md` and `docs/fukurahub-api.md` are normative for
  anything on the wire. Never ship code that contradicts them without
  first updating the spec — and add an ADR under `docs/adr/` when the
  change is load-bearing.
- `docs/implementation-status.md` reflects what has actually shipped
  across the three repos. Bump rows in the same commit that moves the
  code. Reviewers should reject a behaviour change that does not bump
  status.
- `docs/governance.md` is the written-down process for all of the
  above.

## Before coding

1. Read the relevant sections of the spec and
   `docs/implementation-status.md`.
2. If the change is strategic (affects API shape, licensing, cross-
   repo behaviour), draft an ADR first.
3. Check whether the change belongs in the CLI at all. Hub-specific
   work goes in `fukura-hub`; doc rendering goes in `fukura-site`.
4. If testing the hub client path, prefer `tests/hub_http_client.rs`
   — it is the contract-test source of truth for both fukura's own
   CI and fukura-hub's CI (via `HUB_BASE_URL`).

## After coding

1. Update `docs/implementation-status.md` rows touched by the change.
2. Run: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`,
   `cargo test --lib --tests`.
3. Commit messages follow the Conventional Commits style used by
   existing history (`feat(hub):`, `test(hub):`, `docs:`, …).
4. For cross-repo changes, link the sibling PRs in the description.

## Hard rules

- Never include the author's employer name or any internal-platform
  name in code, comments, commit messages, or docs. Use abstract
  terms (`internal platform`, `employer`, `organisation-specific`).
- Never force-push `main` or any published branch without explicit
  approval. History sanitisation is a maintainer decision.
- Never pass `--no-verify`, `--no-gpg-sign`, or similar hook-skipping
  flags unless the user has asked for it.
- Never run destructive DB operations (drop table, mass UPDATE,
  truncate) without an explicit migration plan approved by the
  maintainer.
- Never implement strategic items (licensing, moat, public-facing
  positioning) without prior approval.

## Coordination

- Decisions affecting multiple repos → ADR in `docs/adr/NNNN-*.md`.
- Each repo is tagged independently. There is no shared version
  number; cross-repo coordination is tracked in
  `docs/implementation-status.md`, not in tags.
- Contract tests (`tests/hub_http_client.rs`, run against a real hub
  via `HUB_BASE_URL`) are the final arbiter of spec ↔ server
  alignment. If they fail in `fukura-hub`'s CI, that is the source of
  truth.

## Confidentiality

Before writing any new proper noun that identifies a person, company,
or internal platform, verify with the maintainer. Do not invent
placeholders from training data.
