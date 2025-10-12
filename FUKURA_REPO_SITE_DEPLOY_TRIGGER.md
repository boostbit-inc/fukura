# Fukura Repository: Site Deploy Trigger Setup

This document explains how to set up automatic site deployment triggers when a new release is published in the `fukura` CLI repository.

## Overview

When you release a new version of `fukura` CLI, the `fukura-site` should automatically:
1. Fetch the latest version information
2. Rebuild the site with updated version
3. Deploy to production (Vercel)

## Required Setup in fukura Repository

### 1. Create GitHub Actions Workflow

Create `.github/workflows/notify-site-on-release.yml` in the **fukura** repository:

```yaml
name: Notify Site on Release

on:
  release:
    types: [published]

jobs:
  notify-site:
    name: Trigger Site Deployment
    runs-on: ubuntu-latest
    
    steps:
      - name: Extract version
        id: version
        run: |
          echo "version=${{ github.event.release.tag_name }}" >> $GITHUB_OUTPUT
          
      - name: Trigger fukura-site deployment
        uses: peter-evans/repository-dispatch@v3
        with:
          token: ${{ secrets.SITE_DEPLOY_TOKEN }}
          repository: boostbit-inc/fukura-site
          event-type: release
          client-payload: |
            {
              "version": "${{ github.event.release.tag_name }}",
              "release_url": "${{ github.event.release.html_url }}",
              "published_at": "${{ github.event.release.published_at }}"
            }
      
      - name: Confirmation
        run: |
          echo "✅ Triggered fukura-site deployment for version ${{ steps.version.outputs.version }}"
```

### 2. Create GitHub Personal Access Token (PAT)

You need a PAT with `repo` scope to trigger the `repository_dispatch` event.

#### Steps:
1. Go to GitHub Settings → Developer settings → Personal access tokens → Tokens (classic)
2. Click "Generate new token (classic)"
3. Name: `FUKURA_SITE_DEPLOY_TOKEN`
4. Select scopes:
   - ✅ `repo` (Full control of private repositories)
5. Generate token and copy it

### 3. Add Secret to fukura Repository

1. Go to `fukura` repository → Settings → Secrets and variables → Actions
2. Click "New repository secret"
3. Name: `SITE_DEPLOY_TOKEN`
4. Value: Paste the PAT you created above
5. Click "Add secret"

## How It Works

### Trigger Flow

```
fukura repo: New Release Published
         ↓
GitHub Actions: notify-site-on-release.yml
         ↓
repository_dispatch event → fukura-site
         ↓
fukura-site: site-deploy.yml workflow triggered
         ↓
1. Fetch version from payload
2. Update fukura-release.json
3. Build site
4. Deploy to Vercel
```

### Example Event Payload

When you publish v0.3.3, the fukura-site receives:

```json
{
  "version": "v0.3.3",
  "release_url": "https://github.com/boostbit-inc/fukura/releases/tag/v0.3.3",
  "published_at": "2025-10-10T12:00:00Z"
}
```

## Testing

### Manual Trigger (for testing)

You can manually trigger the site deployment from the fukura repository:

```bash
curl -X POST \
  -H "Accept: application/vnd.github+json" \
  -H "Authorization: Bearer YOUR_PAT_TOKEN" \
  https://api.github.com/repos/boostbit-inc/fukura-site/dispatches \
  -d '{
    "event_type": "release",
    "client_payload": {
      "version": "v0.3.3",
      "release_url": "https://github.com/boostbit-inc/fukura/releases/tag/v0.3.3",
      "published_at": "2025-10-10T12:00:00Z"
    }
  }'
```

Or use GitHub CLI:

```bash
gh api repos/boostbit-inc/fukura-site/dispatches \
  -f event_type=release \
  -f client_payload='{"version":"v0.3.3","release_url":"https://github.com/boostbit-inc/fukura/releases/tag/v0.3.3"}'
```

## Verification

After setting up:

1. Create a test release in fukura repository
2. Check GitHub Actions in fukura → "Notify Site on Release" should run
3. Check GitHub Actions in fukura-site → "Site Deploy" should be triggered
4. Verify the site shows the new version at https://fukura.dev

## Troubleshooting

### Site deployment not triggered

1. **Check PAT token permissions**:
   - Must have `repo` scope
   - Must not be expired

2. **Check workflow file**:
   - Ensure `notify-site-on-release.yml` exists in fukura repo
   - Check syntax is correct

3. **Check secret name**:
   - Must be exactly `SITE_DEPLOY_TOKEN`
   - Case-sensitive

4. **Check GitHub Actions logs**:
   - fukura repo → Actions tab
   - Look for errors in "Notify Site on Release" workflow

### Version not updating on site

1. **Check fukura-site workflow**:
   - Go to fukura-site → Actions
   - Check if "Site Deploy" workflow ran successfully

2. **Check Vercel deployment**:
   - Logs should show the new version
   - Check `public/fukura-release.json` was created

3. **Clear browser cache**:
   - Hard refresh (Ctrl+Shift+R or Cmd+Shift+R)
   - Or open in incognito mode

## Current Status

- ✅ fukura-site: `site-deploy.yml` workflow ready
- ✅ fukura-site: `manual-deploy.yml` workflow ready
- ⏳ fukura repo: Needs `notify-site-on-release.yml` workflow
- ⏳ fukura repo: Needs `SITE_DEPLOY_TOKEN` secret

## Next Steps

1. Add the workflow file to fukura repository
2. Create and add the PAT secret
3. Test with next release
4. Monitor GitHub Actions for successful deployment

