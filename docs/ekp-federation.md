# EKP Federation — v0.1 (Draft)

**Status:** Draft / Request for Comments
**Editor:** Fukura contributors
**License:** CC-BY 4.0
**Relates to:** [EKP v1](./ekp-spec.md) §13, [Hub API v1](./fukurahub-api.md) §13

## 1. Motivation

A fukurahub belongs to one organisation. Within it, fingerprinted
error records and the effectiveness of their fixes accumulate into a
useful corpus: after a few months of use, the *same* fingerprints
recur in the same patterns, and suggestions ranked by measured
success rate outperform text search.

At the scale of the single organisation, this is already the value
fukura sells. What it does not yet do is share the insight *across*
organisations: two companies hitting the same `cargo.compile.e0308`
error have no way to learn from each other&rsquo;s fixes without
exposing their private note bodies, shell invocations, or org names.

Federation is the mechanism that lets that cross-org learning happen
without any of those leaks. This document specifies how.

## 2. Design goals

1. **Zero raw-data leakage.** The only values that ever cross an org
   boundary are EKP fingerprints, outcome counts, and coarse adapter
   metadata. Note titles, bodies, stack traces, entity values, and
   authors stay at their originating hub.
2. **k-anonymity by default.** A fingerprint + outcome pair is only
   redistributed when at least *k* distinct orgs have independently
   reported that fingerprint. Below the threshold, the aggregated
   record is suppressed.
3. **Opt-in per hub.** A hub operator turns federation on explicitly.
   Nothing federates by default. Revocation (stop contributing) is a
   single config flip.
4. **Tamper-evident aggregation.** The aggregator signs each published
   bucket with its own key; hubs verify signatures before trusting.
5. **Spec extension, not spec replacement.** This protocol adds to
   `/v1/*`; it does not change the semantics of existing endpoints.

## 3. Terminology

| Term | Meaning |
| --- | --- |
| **Contributing hub** | A fukurahub that has opted in to federation. |
| **Aggregator** | A trusted third-party service that receives redacted contributions from hubs, enforces k-anonymity, and republishes buckets. |
| **Bucket** | One `(fingerprint, adapter, adapter_version)` triple with its aggregated stats. |
| **k-threshold** | Minimum number of distinct contributing hubs that must report a bucket before the aggregator will publish it. Default: **5**. |
| **Publication window** | The period over which contributions accumulate before publication. Default: **24 hours**. |

## 4. Contribution record

```json
{
  "schema": "fuku.ekp.federation.contribution",
  "version": 1,
  "hub_id_hash": "blake3:…",
  "window_start": "2026-04-18T00:00:00Z",
  "window_end":   "2026-04-19T00:00:00Z",
  "buckets": [
    {
      "fingerprint": "blake3:7f9c…",
      "adapter": "cargo",
      "adapter_version": "0.3.0",
      "success": 12,
      "failure": 3,
      "abandoned": 1
    }
  ]
}
```

- `hub_id_hash` is `BLAKE3(hub_id || aggregator_salt)`. The salt is
  published by the aggregator and rotated per publication window, so
  a hub's sequence of contributions cannot be linked across windows
  without the current salt.
- Each bucket carries only aggregate counts; individual attempts are
  never transmitted.
- `adapter_version` is included because a fingerprint-changing
  adapter release should start a new bucket rather than merging with
  the old one.

## 5. Aggregation

Within one publication window the aggregator:

1. Verifies the contribution signature (§8).
2. For each bucket, increments the running totals keyed on
   `(fingerprint, adapter, adapter_version)`.
3. Tracks the set of distinct `hub_id_hash` values that reported each
   bucket.
4. At window close, **drops** every bucket whose distinct-hub count
   is below the k-threshold, then publishes the remainder.

Buckets above the threshold expose the aggregated counts + the number
of contributing hubs; they do **not** expose the individual
`hub_id_hash` list.

## 6. Publication record

```json
{
  "schema": "fuku.ekp.federation.publication",
  "version": 1,
  "aggregator": "fukurahub-federation.example",
  "window_start": "2026-04-18T00:00:00Z",
  "window_end":   "2026-04-19T00:00:00Z",
  "k_threshold": 5,
  "signature": "ed25519:…",
  "buckets": [
    {
      "fingerprint": "blake3:7f9c…",
      "adapter": "cargo",
      "adapter_version": "0.3.0",
      "success": 127,
      "failure": 44,
      "abandoned": 13,
      "contributing_hubs": 9
    }
  ]
}
```

- `contributing_hubs` is the count only; no identity leaks.
- Buckets with a contributor count **below** `k_threshold` are simply
  absent from the list. Consumers MUST NOT treat absence as proof
  that a fingerprint does not exist in other orgs — it may exist but
  fall under k.

## 7. Transport

- **Push:** contributing hubs `POST /fed/v1/contribute` to the
  aggregator at window close.
- **Pull:** contributing hubs `GET /fed/v1/publications` at
  publication close and merge bucket counts into a read-only
  `federated_stats` table keyed on fingerprint.
- **Client exposure:** hubs surface federated stats to CLI / MCP
  callers as a *separate* block alongside the org-local
  effectiveness block, never blended. Consumers must be able to tell
  per-request which stats came from their own org and which from the
  federation.

## 8. Trust model

- The aggregator is a single trust root per federation. It signs
  publications with an Ed25519 key whose public half is distributed
  out of band (typically checked into the fukura repo alongside a
  manifest of known aggregators).
- Contributing hubs sign their submissions with per-hub Ed25519
  keys; the aggregator verifies before counting. Key rotation is the
  hub operator's responsibility.
- The aggregator is **not** a privacy oracle — it sees unaggregated
  per-hub counts during the window, before publication. Operators
  who distrust the aggregator&rsquo;s operational security should
  not opt in.
- A future extension may move to a verifiable-aggregator design
  (secure multi-party computation or TEE attestation) so the
  aggregator cannot see pre-threshold counts either. Out of scope for
  v0.1.

## 9. Redaction and abuse resistance

- Hubs MUST strip any fingerprint whose adapter does not meet the
  fingerprint-stability requirements of EKP §6 before contributing.
- Hubs MUST NOT contribute buckets they did not produce themselves
  (no pass-through from upstream federations).
- Aggregators MUST discard contributions from a `hub_id_hash` that
  exceeds a configured per-window rate (to prevent a malicious hub
  from inflating bucket counts above k via fake identities).
- Aggregators SHOULD publish their abuse-resistance parameters
  (rate limits, min window size, replay windows) as part of the
  publication record, so consumers can reason about the bucket's
  trustworthiness.

## 10. Privacy analysis

- **Fingerprints are non-reversible.** EKP §6 requires adapters not
  to include variable values (paths, hostnames, UUIDs) in the
  signature. An attacker observing a fingerprint cannot reconstruct
  stderr.
- **Counts are not identifying.** Below the k-threshold, buckets are
  suppressed entirely, so low-volume fingerprints (which might
  identify the contributing org by process of elimination) never
  publish.
- **Hub identity is rotated.** Because `hub_id_hash` changes per
  publication window (see §4), an attacker cannot string together
  an org's contribution fingerprint over time.
- **No note bodies, no entity values, no authors.** The contribution
  record carries none of them. Bugs that would add such fields to
  the schema MUST be treated as privacy incidents and fixed before
  the next publication window.

## 11. Conformance

A contributing hub is federation-conformant if it:

- emits contribution records per §4,
- signs submissions per §8,
- never includes per-attempt records,
- suppresses contributions when the hub operator disables
  federation.

An aggregator is federation-conformant if it:

- enforces the k-threshold per §5,
- rotates the hub-id salt each window,
- publishes only above-threshold buckets,
- signs publications per §8,
- publishes its abuse-resistance parameters.

## 12. Non-goals for v0.1

- **Cross-federation interop.** A hub speaks to one aggregator at a
  time. Mesh topologies wait for a later version.
- **Differential privacy beyond k-anonymity.** Adding calibrated
  noise to the aggregate counts is a valid defence against
  membership inference, but implementing it well (parameter tuning,
  budget tracking) is a separate spec.
- **Negative results.** Fingerprints that *never* fail anywhere do
  not contribute any useful signal; they simply never appear in
  contribution records. Explicit "no-one has ever failed at X"
  signals are not part of this protocol.
- **Reputation of individual fixes.** Aggregation is at the
  fingerprint level only. Which specific `SolutionAttempt.next_command`
  actually worked stays within the org that recorded it.

## 13. Open questions

- Is k=5 the right default, or should it scale with bucket volume?
  Higher-volume fingerprints can tolerate a lower k without losing
  anonymity.
- Should the publication carry a Bloom filter of suppressed
  fingerprints so consumers can detect "this fingerprint did
  contribute but fell under k"? Risk: filter false positives could
  themselves leak info.
- How should adapter-version forks (two forks of the same public
  adapter with diverging fingerprint formulae) be reconciled at
  aggregation time? Currently buckets are keyed on exact
  adapter_version, which shards contributions.
- Federation of attempt *contexts* (agent_kind distributions) —
  useful signal but may re-introduce identifiability.

## 14. Change log

- v0.1 (2026-04-19): Initial draft.
