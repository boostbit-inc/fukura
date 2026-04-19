# ADR 0009: Purge legacy `privacy=private` rows before GA

Date: 2026-04-19
Status: Accepted
Supersedes: [ADR 0005](./0005-private-tier-existing-rows-keep.md)

## Context

ADR 0005 decided to leave `privacy=private` rows in the hub's
`notes` table and enforce rejection only on new uploads going
forward. A security review by a persona-simulation exercise flagged
this as a contract-level red flag: "spec was violated in production,
remediation preserves the violation." That is exactly the kind of
detail a conservative procurement or compliance team asks about on a
sales call, and the answer "they're still in there" lands badly
regardless of how careful the subsequent behaviour is.

The hub currently has no paying customers. Any `private` rows that
exist today are test data or early beta data from design-partner
accounts that can be contacted directly. The window to hard-purge
these rows without producing a real data-loss incident is open now
and closes the moment the first paying customer writes anything.

## Decision

Before GA, **hard-delete every row in `notes` where `privacy =
'private'`**, and log each deletion into `audit_log` for the record.
Existing authors receive an out-of-band email listing the affected
note titles so they can re-capture locally if they want the content
back. After the purge, the hub carries zero private rows; the
spec-level invariant "the hub never stores `private` data" holds
prospectively *and* retrospectively.

Execution:

1. Back up `notes` and `note_versions` (dump to encrypted S3,
   retained 30 days, then destroyed).
2. Run the migration `008_purge_private_rows.sql` — selects affected
   rows into `audit_log` first (`action = 'note.purge.private'`),
   then deletes them and every dependent row (`note_versions`,
   `note_tags`, `comments`, `note_stats`, `bookmarks`). All in one
   transaction so a failure rolls the purge back entirely.
3. Send a notification email to each affected author with the titles
   they lost and instructions for re-capturing locally.
4. Update `implementation-status.md` to note spec §4 is now enforced
   for both new uploads and legacy data.

## Consequences

- The server now satisfies spec §4 ("server MUST reject `private`")
  as a property of the database state, not just of the handler
  behaviour. Any future compliance or SOC 2 audit can inspect the
  row count and see zero.
- A small number of users lose data they had already uploaded. This
  is real harm and we accept it because (a) the affected rows were
  never supposed to exist on the server in the first place and (b)
  the loss is offset by the audit-log entries and the notification
  email, which together give users the option to recover the content
  from their local `.fukura/` store or from their own backups.
- ADR 0005's rollback plan ("a single `DELETE` batch with a clear
  policy trail") is now executed rather than deferred.
- This only works because the customer base is currently tiny. A
  future analogous situation at scale would require per-customer
  opt-in rather than a blanket purge; the affordance for that is
  stored as a follow-up, not committed to.

## Follow-ups

- The migration SQL in `backend/migrations/008_purge_private_rows.sql`
  is the canonical script. Do not hand-edit production rows to
  achieve the same result; use the scripted path so `audit_log`
  captures what happened.
- Run the out-of-band notification script after migration but
  before the next deploy; see `scripts/notify-purged-authors.md` in
  the `fukura-hub` repo.
