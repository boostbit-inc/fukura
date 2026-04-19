//! Seed a hub with realistic demo data.
//!
//! Running a fresh hub against an empty Postgres produces a correct
//! but unconvincing screenshot: every widget reads "no data."
//! `fuku hub seed-demo` populates the hub with a plausible set of
//! errors, fixes, and attempt outcomes so the `/effectiveness`
//! dashboard has something meaningful to show immediately.
//!
//! Why this is a CLI subcommand rather than a server endpoint:
//! - It posts exclusively through the public `/v1/*` API, so it
//!   exercises the same wire contract a real CLI user hits.
//! - It cannot accidentally skip validation or redaction — the data
//!   it writes is indistinguishable from data any other client could
//!   legitimately submit.
//! - It runs in the operator's account, against whatever context
//!   they currently act as. Personal context → `privacy=public`
//!   notes; org context → `privacy=org` notes.
//!
//! The seed is deterministic for a given size so screenshots taken
//! on one developer's machine match a teammate's.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};
use serde_json::{json, Value};

use crate::domain::attempt::{AttemptOutcome, SolutionAttempt};
use crate::domain::models::{Author, Note, NoteEnvelope, Privacy};
use crate::domain::ontology::ErrorOntology;
use crate::hub::{HttpHubClient, HubClient};

pub struct SeedOptions {
    /// Number of distinct fingerprints (≈ notes) to generate. Typical
    /// value: 15–30. Each fingerprint also gets a matching note, so
    /// the table coverage donut reads ~100% out of the box.
    pub notes: usize,
    /// Total attempts to distribute across all fingerprints. Typical
    /// value: 500. Larger makes the KPI tiles impressive but takes
    /// longer to upload.
    pub attempts: usize,
}

impl Default for SeedOptions {
    fn default() -> Self {
        Self {
            notes: 20,
            attempts: 500,
        }
    }
}

pub struct SeedReport {
    pub notes_uploaded: usize,
    pub attempts_uploaded: usize,
    pub fingerprints: Vec<String>,
}

pub async fn seed_demo(client: &HttpHubClient, opts: SeedOptions) -> Result<SeedReport> {
    let specs = fingerprint_catalog();
    let count = opts.notes.min(specs.len());
    let picked = &specs[..count];

    // --- Notes ---
    let now = Utc::now();
    let mut fingerprints: Vec<String> = Vec::with_capacity(picked.len());
    let mut notes_uploaded = 0usize;
    for (idx, spec) in picked.iter().enumerate() {
        let ontology = ErrorOntology::new(spec.adapter, spec.category, spec.fingerprint);
        let note = Note {
            title: spec.title.to_string(),
            body: spec.body.to_string(),
            tags: spec.tags.iter().map(|s| (*s).to_string()).collect(),
            links: Vec::new(),
            meta: BTreeMap::new(),
            solutions: Vec::new(),
            privacy: Privacy::Public,
            created_at: now - ChronoDuration::days(7) + ChronoDuration::hours(idx as i64),
            updated_at: now - ChronoDuration::hours(idx as i64),
            author: Author {
                name: "fukura demo".into(),
                email: None,
            },
            ontology: Some(ontology),
        };
        let envelope = NoteEnvelope {
            schema: "fuku.note".into(),
            version: 1,
            note,
        };
        match client.upload_note(&envelope).await {
            Ok(_) => notes_uploaded += 1,
            Err(err) => {
                tracing::warn!("upload_note for {} failed: {err:?}", spec.fingerprint);
            }
        }
        fingerprints.push(spec.fingerprint.to_string());
    }

    // --- Attempts ---
    // Distribute attempts across fingerprints with a power-law-ish
    // weight so a handful of problems dominate (matches real teams).
    let weights: Vec<u32> = (0..picked.len())
        .map(|i| (picked.len() - i) as u32 + 2)
        .collect();
    let total_weight: u32 = weights.iter().sum();
    let mut per_fp: Vec<usize> = weights
        .iter()
        .map(|w| (opts.attempts as u64 * *w as u64 / total_weight as u64) as usize)
        .collect();
    // Fix rounding so the total matches the request exactly.
    let drift: i64 = opts.attempts as i64 - per_fp.iter().sum::<usize>() as i64;
    if drift != 0 {
        if drift > 0 {
            per_fp[0] += drift as usize;
        } else if let Some(first) = per_fp.first_mut() {
            *first = first.saturating_sub((-drift) as usize);
        }
    }

    let mut attempts_uploaded = 0usize;
    for (idx, spec) in picked.iter().enumerate() {
        let n = per_fp[idx];
        if n == 0 {
            continue;
        }
        let batch = generate_attempts(spec, n, now);
        // Upload in chunks of 200 to stay well under the hub's
        // max_attempts_per_batch limit (500) even if the server has a
        // stricter bespoke value.
        for chunk in batch.chunks(200) {
            match client.upload_attempts(chunk).await {
                Ok(r) => attempts_uploaded += r.accepted as usize,
                Err(err) => {
                    tracing::warn!(
                        "upload_attempts for {} failed: {err:?}",
                        spec.fingerprint
                    );
                }
            }
            // Small pause so we don't trip the hub's rate limiter in a
            // single burst — 600 req/min = 10/s, batches of 200 are
            // well under that.
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    Ok(SeedReport {
        notes_uploaded,
        attempts_uploaded,
        fingerprints,
    })
}

fn generate_attempts(
    spec: &FingerprintSpec,
    count: usize,
    now: chrono::DateTime<Utc>,
) -> Vec<SolutionAttempt> {
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let outcome = pick_outcome(spec.target_success_rate, i);
        let mut a = SolutionAttempt::new(spec.fingerprint, outcome);
        a.agent_kind = pick_agent_kind(i);
        a.suggested_note_id = None;
        a.next_command = match outcome {
            AttemptOutcome::Success => Some(spec.fix_command.to_string()),
            _ => None,
        };
        // Spread occurrence timestamps evenly across the last 7 days
        // so the time-range tabs show a sensible volume in every
        // window.
        a.occurred_at =
            now - ChronoDuration::minutes((count - i) as i64 * (7 * 24 * 60 / count.max(1) as i64));
        out.push(a);
    }
    out
}

fn pick_outcome(target_success_rate: f32, idx: usize) -> AttemptOutcome {
    // Interleave outcomes deterministically so the rate converges to
    // `target_success_rate` without needing a PRNG.
    let bucket = (idx as f32 * target_success_rate).floor() as u32;
    let prev = ((idx.saturating_sub(1)) as f32 * target_success_rate).floor() as u32;
    if bucket > prev {
        AttemptOutcome::Success
    } else if idx % 11 == 0 {
        AttemptOutcome::Abandoned
    } else {
        AttemptOutcome::Failure
    }
}

fn pick_agent_kind(idx: usize) -> Option<String> {
    // 60% agent / 40% human; among agents, 55% claude-code, 35% cursor,
    // 10% devin. Rough mirror of a team that has rolled out Claude Code
    // with some Cursor holdouts.
    match idx % 20 {
        0..=7 => None, // human (40%)
        8..=14 => Some("claude-code".into()), // 35%
        15..=18 => Some("cursor".into()),     // 20%
        _ => Some("devin".into()),            // 5%
    }
}

struct FingerprintSpec {
    adapter: &'static str,
    category: &'static str,
    fingerprint: &'static str,
    title: &'static str,
    body: &'static str,
    tags: &'static [&'static str],
    fix_command: &'static str,
    /// How often the "right" fix actually works. Lower means "this is
    /// a trickier bug" and pushes it up the pain-rank.
    target_success_rate: f32,
}

/// 24 fingerprints across the common failure modes a Rust/K8s/Web team
/// hits every week. Deterministic — two seeds produce identical output.
fn fingerprint_catalog() -> Vec<FingerprintSpec> {
    vec![
        FingerprintSpec {
            adapter: "cargo", category: "cargo.compile.e0432",
            fingerprint: "blake3:demo-cargo-e0432-unresolved-import",
            title: "cargo: unresolved import in workspace member",
            body: "When a new workspace member is added but the root Cargo.toml hasn't re-picked up the members list, `cargo build` fails with E0432. Run `cargo metadata` then re-run the build.",
            tags: &["cargo", "rust", "workspace"],
            fix_command: "cargo metadata && cargo build",
            target_success_rate: 0.82,
        },
        FingerprintSpec {
            adapter: "cargo", category: "cargo.compile.e0308",
            fingerprint: "blake3:demo-cargo-e0308-mismatched-types",
            title: "cargo: mismatched types across async boundary",
            body: "Usually an Error type that doesn't implement Send across the .await point. Wrap the non-Send value in a scope that ends before the await.",
            tags: &["cargo", "rust", "async"],
            fix_command: "cargo check",
            target_success_rate: 0.55,
        },
        FingerprintSpec {
            adapter: "cargo", category: "cargo.lock.poisoned",
            fingerprint: "blake3:demo-cargo-lock-poisoned",
            title: "cargo build: Cargo.lock out of sync with Cargo.toml",
            body: "Someone added a dependency without running `cargo check`; the lockfile is stale and CI refuses. `cargo update -p <crate>` or `cargo generate-lockfile` if the drift is big.",
            tags: &["cargo", "rust", "lockfile"],
            fix_command: "cargo generate-lockfile",
            target_success_rate: 0.91,
        },
        FingerprintSpec {
            adapter: "cargo", category: "cargo.test.flaky",
            fingerprint: "blake3:demo-cargo-test-flaky-tokio",
            title: "cargo test: flaky tokio multi-thread test",
            body: "A test using `#[tokio::test]` with `flavor = multi_thread` that intermittently fails on shared state. Add `--test-threads=1` when repro is needed, then actually fix the race.",
            tags: &["cargo", "rust", "tokio", "flaky"],
            fix_command: "cargo test -- --test-threads=1",
            target_success_rate: 0.38,
        },
        FingerprintSpec {
            adapter: "git", category: "git.merge.conflict.rebase",
            fingerprint: "blake3:demo-git-rebase-conflict",
            title: "git rebase: conflict in Cargo.lock during long rebase",
            body: "Cargo.lock changes in every feature branch; a long rebase triples the conflict count. Regenerate after the rebase completes: `git checkout --theirs Cargo.lock && cargo check`.",
            tags: &["git", "rebase", "cargo"],
            fix_command: "git checkout --theirs Cargo.lock && cargo check",
            target_success_rate: 0.88,
        },
        FingerprintSpec {
            adapter: "git", category: "git.push.rejected",
            fingerprint: "blake3:demo-git-push-rejected-ff",
            title: "git push: rejected — non-fast-forward",
            body: "Remote has moved since your last fetch. `git pull --rebase` and re-push. If it's a protected branch, open a PR instead.",
            tags: &["git", "push"],
            fix_command: "git pull --rebase && git push",
            target_success_rate: 0.95,
        },
        FingerprintSpec {
            adapter: "git", category: "git.hook.pre-commit-fail",
            fingerprint: "blake3:demo-git-hook-precommit",
            title: "git commit: pre-commit hook fails on unstaged formatter changes",
            body: "The formatter wrote files the commit doesn't include. Re-stage: `git add -u && git commit`.",
            tags: &["git", "hook", "fmt"],
            fix_command: "cargo fmt && git add -u && git commit",
            target_success_rate: 0.80,
        },
        FingerprintSpec {
            adapter: "kubernetes", category: "kubernetes.image_pull.backoff",
            fingerprint: "blake3:demo-k8s-imagepullbackoff",
            title: "kubectl apply: ImagePullBackOff after tag push",
            body: "Image was pushed but the tag the manifest references has been retagged. `kubectl rollout restart` only helps if the registry can still resolve the digest.",
            tags: &["kubernetes", "image-pull"],
            fix_command: "kubectl rollout restart deploy/<name>",
            target_success_rate: 0.42,
        },
        FingerprintSpec {
            adapter: "kubernetes", category: "kubernetes.pod.crashloop",
            fingerprint: "blake3:demo-k8s-crashloop-oom",
            title: "kubectl: pod CrashLoopBackOff, OOMKilled in logs",
            body: "Container memory limit set too low for the new request volume. Bump limits + raise requests in the Deployment, or fix the leak.",
            tags: &["kubernetes", "oom"],
            fix_command: "kubectl edit deploy <name>",
            target_success_rate: 0.60,
        },
        FingerprintSpec {
            adapter: "kubernetes", category: "kubernetes.rbac.forbidden",
            fingerprint: "blake3:demo-k8s-rbac-forbidden",
            title: "kubectl: 'forbidden: cannot list resource X' after cluster upgrade",
            body: "API version changed on the cluster, RBAC role still references the old group. Patch the ClusterRole.",
            tags: &["kubernetes", "rbac"],
            fix_command: "kubectl edit clusterrole <name>",
            target_success_rate: 0.70,
        },
        FingerprintSpec {
            adapter: "generic", category: "generic.terraform.state-lock",
            fingerprint: "blake3:demo-terraform-state-lock",
            title: "terraform apply: state lock held by abandoned CI job",
            body: "Previous CI run died without releasing the S3 state lock. `terraform force-unlock <id>` after confirming nothing is actually running.",
            tags: &["terraform", "state"],
            fix_command: "terraform force-unlock <id>",
            target_success_rate: 0.85,
        },
        FingerprintSpec {
            adapter: "generic", category: "generic.docker.buildx-cache",
            fingerprint: "blake3:demo-docker-buildx-cache",
            title: "docker build: cache mount permission denied on CI",
            body: "`--mount=type=cache` requires the dockerfile frontend + registered cache. Use `--cache-from type=registry` in CI instead.",
            tags: &["docker", "buildx"],
            fix_command: "docker buildx build --cache-from type=registry,ref=... .",
            target_success_rate: 0.50,
        },
        FingerprintSpec {
            adapter: "generic", category: "generic.npm.peer-dep",
            fingerprint: "blake3:demo-npm-peer-dep",
            title: "npm install: peer dep conflict after major bump",
            body: "Two transitive packages pin different majors of the same peer. Use `--legacy-peer-deps` to unblock the install, then replace one of the conflicting packages.",
            tags: &["npm", "peer-deps"],
            fix_command: "npm install --legacy-peer-deps",
            target_success_rate: 0.75,
        },
        FingerprintSpec {
            adapter: "generic", category: "generic.pg.connection-refused",
            fingerprint: "blake3:demo-pg-connection-refused",
            title: "psql: connection refused from CI against a managed Postgres",
            body: "Managed DB sometimes rotates its public endpoint; CI's DATABASE_URL still points at the old DNS. Refresh the secret.",
            tags: &["postgres", "ci"],
            fix_command: "rotate-db-secret.sh && ci/rerun.sh",
            target_success_rate: 0.65,
        },
        FingerprintSpec {
            adapter: "generic", category: "generic.dns.resolv",
            fingerprint: "blake3:demo-dns-nxdomain",
            title: "curl: NXDOMAIN inside container only",
            body: "Container DNS is using `/etc/resolv.conf` written before the VPN came up. Restart the container or override with `--dns`.",
            tags: &["dns", "docker"],
            fix_command: "docker run --dns=1.1.1.1 ...",
            target_success_rate: 0.72,
        },
        FingerprintSpec {
            adapter: "generic", category: "generic.tls.cert-expired",
            fingerprint: "blake3:demo-tls-expired",
            title: "curl: TLS handshake failure — cert expired on internal service",
            body: "An internal service's cert expired. cert-manager should have renewed but didn't. Force renewal.",
            tags: &["tls", "cert-manager"],
            fix_command: "kubectl delete certificaterequest -n <ns> ...",
            target_success_rate: 0.55,
        },
        FingerprintSpec {
            adapter: "cargo", category: "cargo.publish.rejected",
            fingerprint: "blake3:demo-cargo-publish-rejected",
            title: "cargo publish: version already exists on crates.io",
            body: "Previous release workflow half-succeeded (tagged locally, published to crates.io, then failed on docs). Bump patch and retry.",
            tags: &["cargo", "publish", "release"],
            fix_command: "cargo release patch --execute",
            target_success_rate: 0.88,
        },
        FingerprintSpec {
            adapter: "generic", category: "generic.ci.auth-expired",
            fingerprint: "blake3:demo-ci-auth-expired",
            title: "CI: GitHub Actions OIDC auth to cloud provider failed",
            body: "Trust policy on the cloud-side role no longer matches the repo after a rename. Re-grant the `sub` claim.",
            tags: &["ci", "github-actions", "oidc"],
            fix_command: "update-oidc-trust.sh",
            target_success_rate: 0.68,
        },
        FingerprintSpec {
            adapter: "generic", category: "generic.node.heap-oom",
            fingerprint: "blake3:demo-node-heap-oom",
            title: "next build: JavaScript heap out of memory",
            body: "Large MDX tree blows the default 1.5 GB heap. Bump with `NODE_OPTIONS=--max-old-space-size=4096`.",
            tags: &["node", "next", "oom"],
            fix_command: "NODE_OPTIONS=--max-old-space-size=4096 npm run build",
            target_success_rate: 0.92,
        },
        FingerprintSpec {
            adapter: "git", category: "git.lfs.missing",
            fingerprint: "blake3:demo-git-lfs-missing",
            title: "git checkout: LFS objects missing on fresh clone",
            body: "`git lfs install` was skipped in the post-clone flow. Run it once per machine, then `git lfs pull`.",
            tags: &["git", "lfs"],
            fix_command: "git lfs install && git lfs pull",
            target_success_rate: 0.90,
        },
    ]
}

/// Turn a `SeedReport` into a Value for JSON output, kept here so the
/// CLI handler stays light on serde.
pub fn report_to_json(r: &SeedReport) -> Value {
    json!({
        "notes_uploaded": r.notes_uploaded,
        "attempts_uploaded": r.attempts_uploaded,
        "fingerprints": r.fingerprints.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_enough_entries_for_default_seed() {
        let specs = fingerprint_catalog();
        assert!(specs.len() >= SeedOptions::default().notes);
    }

    #[test]
    fn pick_outcome_converges_to_target_rate() {
        let n = 1000;
        let target = 0.7;
        let mut success = 0;
        for i in 0..n {
            if matches!(pick_outcome(target, i), AttemptOutcome::Success) {
                success += 1;
            }
        }
        let rate = success as f32 / n as f32;
        assert!(
            (rate - target).abs() < 0.05,
            "expected {target}, got {rate}"
        );
    }
}
