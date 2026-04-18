# Architecture Decision Records

See `docs/governance.md` for the process. Format: Context, Decision,
Consequences, one page max. Numbering is sequential; superseding is
done by adding a new ADR and flipping the old one's status to
`Superseded by ADR-NNNN`.

## Index

| # | Title | Status |
|---|-------|--------|
| [0001](./0001-ekp-as-interop-standard.md) | EKP as the interoperability standard | Accepted |
| [0002](./0002-open-core-license-line.md) | Open-core line between CLI and hub | Accepted |
| [0003](./0003-twolayer-v1-api-coexistence.md) | `/v1/` and `/api/` coexist on the same hub | Accepted |
| [0004](./0004-org-scoping-via-jwt-claim.md) | Org scoping via Bearer token claim | Accepted |
| [0005](./0005-private-tier-existing-rows-keep.md) | Existing `privacy=private` rows are kept on the server | Accepted |
| [0006](./0006-m6-zero-downtime-three-stage.md) | Rename the `team` privacy enum to `org` via a 3-stage deploy | Accepted |
