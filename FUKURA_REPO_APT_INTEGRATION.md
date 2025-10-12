# Fukura Repository: APT Repository Integration Guide

This document outlines the changes needed in the `fukura` repository to enable APT repository hosting on `fukura-site` (fukura.dev).

## 📋 Overview

The `fukura-site` repository now has APT repository infrastructure ready at `https://fukura.dev/apt`. To complete the integration, the `fukura` repository needs to:

1. Export the GPG public key
2. Package the APT repository artifacts
3. Trigger the deployment to fukura-site

## 🔑 1. Export GPG Public Key

### Location
The GPG public key should be uploaded to GitHub Releases so fukura-site can serve it.

### Implementation

Add this step to `.github/workflows/release.yml`:

```yaml
- name: Export GPG public key
  env:
    LINUX_GPG_KEY: ${{ secrets.LINUX_GPG_KEY }}
  run: |
    if [ -n "${LINUX_GPG_KEY:-}" ]; then
      echo "$LINUX_GPG_KEY" | base64 --decode | gpg --batch --import
      KEY_ID=$(gpg --batch --list-secret-keys --with-colons | awk -F: '/^sec:/ {print $5; exit}')
      
      if [ -n "$KEY_ID" ]; then
        # Export public key in ASCII armor format
        gpg --batch --armor --export "$KEY_ID" > dist/fukura-gpg.asc
        echo "Exported GPG public key (Key ID: $KEY_ID)"
      fi
    fi
```

**File to create**: `dist/fukura-gpg.asc`

## 📦 2. Package APT Repository Artifacts

### What to Package

The APT repository structure created by `scripts/linux/build-apt-repo.sh` needs to be packaged and uploaded.

### Implementation

Add this step to `.github/workflows/release.yml` after the APT repository is built:

```yaml
- name: Package APT repository
  run: |
    if [ -d "dist/apt" ]; then
      # Create tarball of APT repository
      cd dist
      tar -czf apt-repo.tar.gz apt/
      cd ..
      
      echo "APT repository packaged: dist/apt-repo.tar.gz"
      
      # List contents for verification
      tar -tzf dist/apt-repo.tar.gz | head -20
    else
      echo "No APT repository found, skipping"
    fi
```

**File to create**: `dist/apt-repo.tar.gz`

### Expected Structure

```
apt-repo.tar.gz
└── apt/
    ├── dists/
    │   └── stable/
    │       ├── Release
    │       ├── Release.gpg
    │       └── main/
    │           ├── binary-amd64/
    │           │   ├── Packages
    │           │   └── Packages.gz
    │           └── binary-arm64/
    │               ├── Packages
    │               └── Packages.gz
    ├── pool/
    │   └── main/
    │       └── f/
    │           └── fukura/
    │               ├── fukura_*.deb
    │               └── fukura_*.deb.asc
    └── fukura-archive-keyring.gpg
```

## 🚀 3. Upload to GitHub Releases

### Implementation

Modify the upload step in `.github/workflows/release.yml` to include the new artifacts:

```yaml
- name: Upload release artifacts
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    # Upload all artifacts including APT repository
    cargo dist upload --from-local dist --owner boostbit-inc --repo fukura --tag ${{ github.ref_name }}
    
    # Verify APT artifacts were uploaded
    echo "Checking uploaded artifacts..."
    gh release view ${{ github.ref_name }} --json assets --jq '.assets[].name' | grep -E "(apt-repo|fukura-gpg)"
```

**Files to upload**:
- `apt-repo.tar.gz`
- `fukura-gpg.asc`

## 📡 4. Trigger fukura-site Deployment

### Implementation

Add this step at the end of `.github/workflows/release.yml`:

```yaml
- name: Trigger fukura-site APT deployment
  env:
    SITE_DISPATCH_TOKEN: ${{ secrets.SITE_DISPATCH_TOKEN }}
  run: |
    if [ -z "${SITE_DISPATCH_TOKEN:-}" ]; then
      echo "SITE_DISPATCH_TOKEN not set, skipping APT deployment"
      exit 0
    fi
    
    echo "Triggering APT repository deployment to fukura-site..."
    curl -X POST \
      -H "Accept: application/vnd.github+json" \
      -H "Authorization: Bearer ${SITE_DISPATCH_TOKEN}" \
      -H "Content-Type: application/json" \
      https://api.github.com/repos/boostbit-inc/fukura-site/dispatches \
      -d '{"event_type":"apt-deploy","client_payload":{"source":"boostbit-inc/fukura","version":"${{ github.ref_name }}","tag":"${{ github.ref_name }}"}}'
    
    echo "✓ APT deployment triggered"
```

**Note**: This uses the existing `SITE_DISPATCH_TOKEN` secret.

## 🔧 5. Complete Workflow Integration

### Suggested Order in `release.yml`

```yaml
jobs:
  release:
    name: Publish GitHub Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        # ... existing steps ...
      
      # Build APT repository (existing)
      - name: Build APT repository skeleton
        run: |
          scripts/linux/build-apt-repo.sh dist stable amd64
      
      # NEW: Export GPG public key
      - name: Export GPG public key
        env:
          LINUX_GPG_KEY: ${{ secrets.LINUX_GPG_KEY }}
        run: |
          # ... (from section 1)
      
      # NEW: Package APT repository
      - name: Package APT repository
        run: |
          # ... (from section 2)
      
      # Upload artifacts (modify existing)
      - name: Upload release artifacts
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          # ... (from section 3)
      
      # Docker build (existing)
      - name: Build and push Docker image
        # ... existing step ...
      
      # Site notification (existing)
      - name: Notify fukura-site of new release
        # ... existing step ...
      
      # NEW: Trigger APT deployment
      - name: Trigger fukura-site APT deployment
        env:
          SITE_DISPATCH_TOKEN: ${{ secrets.SITE_DISPATCH_TOKEN }}
        run: |
          # ... (from section 4)
```

## ✅ Verification Checklist

After implementing these changes, verify:

- [ ] `fukura-gpg.asc` is in GitHub Releases
- [ ] `apt-repo.tar.gz` is in GitHub Releases
- [ ] GitHub Release assets include both files
- [ ] fukura-site receives the `apt-deploy` event
- [ ] APT repository is accessible at `https://fukura.dev/apt`
- [ ] `curl -fsSL https://fukura.dev/fukura-gpg.asc` returns the GPG key

## 🧪 Testing

### Test APT Installation

On a Debian/Ubuntu system:

```bash
# Add GPG key
curl -fsSL https://fukura.dev/fukura-gpg.asc | sudo gpg --dearmor -o /usr/share/keyrings/fukura-archive-keyring.gpg

# Add repository
echo "deb [signed-by=/usr/share/keyrings/fukura-archive-keyring.gpg] https://fukura.dev/apt stable main" | sudo tee /etc/apt/sources.list.d/fukura.list

# Install
sudo apt update
sudo apt install fukura

# Verify
fukura --version
```

### Test Install Script

```bash
curl -sSL https://fukura.dev/install.sh | bash
```

On Debian/Ubuntu, this should:
1. Detect the OS
2. Add the APT repository
3. Install via `apt install fukura`

## 📝 Summary

**New files to create in fukura repository:**
- None (only modify existing `.github/workflows/release.yml`)

**New artifacts to upload:**
- `fukura-gpg.asc` - GPG public key
- `apt-repo.tar.gz` - Complete APT repository structure

**New GitHub Actions event to trigger:**
- `repository_dispatch` with type `apt-deploy` to fukura-site

**Existing secrets to use:**
- `LINUX_GPG_KEY` - Already exists for package signing
- `SITE_DISPATCH_TOKEN` - Already exists for site notifications

---

**Implementation Priority**: High  
**Estimated Time**: 30 minutes  
**Dependencies**: Existing GPG signing infrastructure  
**Testing Required**: Yes (on Debian/Ubuntu VM or container)

