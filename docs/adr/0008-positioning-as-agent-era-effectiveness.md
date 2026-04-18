# ADR 0008: Position fukura as agent-era effectiveness tooling, not a note store

Date: 2026-04-19
Status: Accepted

## Context

Earlier positioning material (handoff.md §2, early site copy, the
`/docs/mcp` page) framed fukura as "the shared memory layer for
agents that bang on tools." That framing is accurate but it does not
tell a buyer what to pay for. Reviewers who worked the product
through a strategy lens landed on a sharper frame:

- The wedge is not knowledge sharing. It is **measured effectiveness
  of an agent fleet**, delivered to a concrete buyer persona who is
  already being asked to justify their agent spend.
- The buyer is not "any engineering team." It is a **Head of
  Developer Platform or DevEx lead at a 100–400-engineer company
  that has already rolled out Claude Code or Cursor org-wide within
  the last 12 months**. Startups are fast to try but slow to pay;
  big-platform VPs have sales cycles that kill small vendors.
- The moat is not the schema. EKP is CC-BY exactly so others can
  adopt it. The defensible positions are (a) cross-vendor data
  nobody with a single agent footprint can collect, and (b) the
  attempt-outcome loop, which is the hard engineering, not the note
  store.
- The demo that converts is not "look, you can search your errors."
  It is **"your Claude Code fleet burned $X retrying the same six
  errors last week; here's which fixes worked"** — numbers on a
  dashboard, ranked.

Without writing this down, future sessions drift back to "knowledge
base for devs," which is a crowded, low-WTP market.

## Decision

Fukura is positioned as **measured effectiveness for agent-era
software teams**. Every product surface — home page, pricing copy,
hub dashboard, demo script — leads with the measurement story. The
knowledge-base aspect is a *consequence* of the measurement, never
the headline.

Concretely this means:

1. **The hub's landing screen is `/effectiveness`**, not a blog-style
   dashboard. Rank fingerprints by pain score
   (`attempts × (1 − success_rate)`), split human vs. agent outcomes,
   flag recurring patterns that lack a linked note, and include a
   live feed so the screen feels like it's watching the work happen.
2. **Site hero copy leads with the signal-volume asymmetry**
   ("agents generate 100–1000× more error signal than humans") and
   the measurement payoff. The "shared knowledge" angle moves to
   supporting text.
3. **Pricing tiers meter on seats + attempt volume**, never on
   captured records or retention. The thing we want customers to
   scale is exactly the thing metering-on-records would penalise.
4. **Competitor framing is concrete**, not generic. Every rival
   paragraph names the specific mechanism missing (e.g. "Sentry has
   no service identity for an agent running `terraform plan` in a
   dev sandbox"). Vague "we're different" language is refused.
5. **The moat story is honest.** Schemas are published, so we don't
   sell the schema. We sell the cross-vendor data aggregation and
   the effectiveness loop, and we say so out loud.

## Consequences

- Future PRs to the site / hub / CLI are reviewed against "does this
  surface or amplify measured effectiveness?" rather than "does this
  help users save snippets?" The latter is a fine *feature*, but no
  longer a positioning goal.
- Competitor surveillance focuses on Claude Code, Cursor, and any
  vendor that ships "team memory with success rates." If that
  feature lands in Cursor or Anthropic, the wedge narrows — and the
  right defence is accelerated cross-vendor coverage, not more
  features inside fukura.
- Marketing hires are briefed on this ADR, not on the earlier handoff
  framing. When the two conflict, this one wins.
- The `/docs/why-fukura` page is the canonical prose expression of
  this decision; if we change positioning, that page and this ADR
  move together.
