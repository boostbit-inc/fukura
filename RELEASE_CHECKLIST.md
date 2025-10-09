# Release Checklist

Use this checklist before creating any release to ensure quality and consistency.

## Pre-Release Checks

### 1. Code Quality
- [ ] Run `cargo fmt --all` to format code
- [ ] Run `cargo fmt --all -- --check` to verify formatting
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Fix all clippy warnings

### 2. Testing
- [ ] Run `cargo test --all` and ensure all tests pass
- [ ] Run `cargo test --test integration`
- [ ] Run `cargo test --test security`
- [ ] Run `cargo test --test performance`
- [ ] Manually test key workflows

### 3. Documentation
- [ ] Update version in `Cargo.toml`
- [ ] Update `README.md` if needed
- [ ] Update `CHANGELOG.md` (if exists)
- [ ] Review all documentation for accuracy

### 4. Build Verification
- [ ] Run `cargo build --release`
- [ ] Test release binary: `cargo install --path . --force`
- [ ] Verify `--version` output
- [ ] Test `--help` for all commands

### 5. Git Workflow
- [ ] Commit all changes
- [ ] Ensure working directory is clean (`git status`)
- [ ] Create annotated tag: `git tag -a vX.Y.Z -m "Release vX.Y.Z: description"`
- [ ] Push commits first: `git push origin main`
- [ ] Push tag: `git push origin vX.Y.Z`

## Release Command Sequence

```bash
# 1. Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# 2. Test
cargo test --all

# 3. Build and verify
cargo build --release
cargo install --path . --force
fukura --version

# 4. Commit (if changes)
git add -A
git commit -m "chore: prepare release vX.Y.Z"

# 5. Tag and push
git tag -a vX.Y.Z -m "Release vX.Y.Z: description"
git push origin main
git push origin vX.Y.Z
```

## Post-Release

- [ ] Verify GitHub Actions CI/CD completes successfully
- [ ] Check GitHub Release page
- [ ] Test installation from release artifacts
- [ ] Announce release (if applicable)

## If CI/CD Fails

1. **DO NOT** force push or delete tags on main branch
2. Fix the issue locally
3. Increment patch version (e.g., v0.3.0 → v0.3.1)
4. Follow the checklist again
5. Document what went wrong for future reference

## Common Mistakes to Avoid

- ❌ Tagging before final commit
- ❌ Skipping `cargo fmt --check`
- ❌ Not running full test suite
- ❌ Force pushing tags
- ❌ Pushing without local verification

## Best Practices

- ✅ Always run full checklist
- ✅ Test on clean checkout
- ✅ Automated checks in CI/CD
- ✅ Keep release notes updated
- ✅ Semantic versioning (MAJOR.MINOR.PATCH)

