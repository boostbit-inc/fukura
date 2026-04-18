# ADR 0004: Org scoping via Bearer token claim

Date: 2026-04-18
Status: Accepted

## Context

The API spec (§3) says tokens are opaque to the client. The spec does
not prescribe how the server identifies which organisation a request
belongs to. Two mechanisms are available:

1. **URL parameter**: `/v1/notes?organization_id=...` (or path). The
   client has to know, pick, and send its org every request.
2. **Token claim**: the token embeds `org_id`; the server extracts it
   during auth and scopes the request automatically.

Option 1 leaks organisation structure into every URL, forces clients
to track an identity they shouldn't need, and makes cross-org
confusion easy — a misconfigured script can send the wrong org and
silently write to the wrong place. Option 2 keeps the org choice at
token-issuance time, where the UI has full context and can present
the user with an unambiguous switcher.

The hub already uses JWTs (via the `jsonwebtoken` crate) so embedding
an additional claim costs nothing.

## Decision

Embed `org_id` (and, when relevant, `role`) as a claim in every issued
token. The `/v1/*` middleware:

1. Validates the `Authorization: Bearer` header.
2. Extracts the `org_id` claim.
3. Puts `OrgId` in the request extensions.
4. Handlers read the extension and scope DB queries by it.

If a user belongs to multiple orgs, they re-issue a token for the org
they want to operate as (UI-side switch). Clients never see claim
internals; tokens stay opaque.

Token issuance remains `/api/auth/*` (the product surface; see ADR
0003). `/v1/*` only consumes tokens, never mints them.

## Consequences

- Accidentally sending the wrong org is reduced to "user picked the
  wrong token in the UI switcher" — still possible but a single point
  of attention rather than a per-request field.
- Tokens are not portable between orgs. A user watching two orgs
  keeps two tokens and rotates by re-login.
- Auth middleware is the only place that learns new claim shapes.
  Handlers remain org-agnostic beyond reading the extension.
- If federation (future) requires per-federation claims, the same
  mechanism extends. The spec's "tokens are opaque" contract is not
  violated.
