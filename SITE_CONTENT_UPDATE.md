# Fukura Site Content Update

> **このファイルはfukura-siteリポジトリへのコピー用です。コピー後削除してください。**

## Hero Section

### Title
```
Fukura
```

### Tagline
```
Capture, Search, and Never Forget Your Error Fixes
```

### Description
```
A Rust CLI for capturing recurring error fixes in a content-addressable store, with fast search (Tantivy), redaction rules, pack files, and a TUI.
```

### Features Highlights (4 cards)

#### 1. Automatic Error Capture
**Icon**: 🎯 (or target icon)
**Title**: Automatic Error Capture
**Description**: 
```
Daemon automatically captures errors and solutions as you develop, creating a comprehensive knowledge base without manual effort.
```
**Bullet Points**:
- Background daemon monitors your development environment
- Captures error messages and successful solutions
- Auto-generates notes after 5 minutes of inactivity
- Zero-configuration setup with intelligent defaults
- Shell hooks for seamless integration (bash/zsh/fish/powershell)

#### 2. Lightning-Fast Search
**Icon**: ⚡ (or lightning icon)
**Title**: Lightning-Fast Search
**Description**:
```
Powered by Tantivy search engine for instant retrieval of relevant error fixes and solutions.
```
**Bullet Points**:
- Sub-second search across thousands of notes
- Full-text search with relevance scoring
- Tag-based filtering and categorization
- Fuzzy matching for typos and variations
- Smart shortcuts: @latest, @1, @2 for quick access

#### 3. Beautiful TUI Interface
**Icon**: 🖥️ (or terminal icon)
**Title**: Beautiful TUI Interface
**Description**:
```
Elegant terminal user interface with multi-pane layout for efficient navigation and editing.
```
**Bullet Points**:
- Multi-pane interface with Tab navigation
- Syntax highlighting for code snippets
- Interactive search with live filtering
- Keyboard shortcuts for power users
- Export to beautiful HTML for browser viewing

#### 4. Security & Privacy First
**Icon**: 🔒 (or shield icon)
**Title**: Security & Privacy First
**Description**:
```
Automatic secret redaction and privacy-first design protect your sensitive information.
```
**Bullet Points**:
- Auto-redacts AWS keys, API tokens, passwords, and more
- 14+ security patterns built-in
- All notes private by default
- Local-first storage in `.fukura/`
- No telemetry or data collection

---

## Quick Start Section

### Step 1: Install
**Platform-specific install commands (auto-detect)**

**Linux / macOS / WSL2**:
```bash
curl -LO https://github.com/boostbit-inc/fukura/releases/latest/download/fukura-x86_64-unknown-linux-gnu.tar.xz
tar -xf fukura-x86_64-unknown-linux-gnu.tar.xz
sudo mv fukura fuku /usr/local/bin/
```

**Windows**:
```powershell
curl -LO https://github.com/boostbit-inc/fukura/releases/latest/download/fukura-x86_64-pc-windows-msvc.zip
Expand-Archive fukura-x86_64-pc-windows-msvc.zip -DestinationPath C:\Tools\fukura
# Add to PATH
```

### Step 2: Initialize
```bash
fuku init
```

### Step 3: Use
```bash
# Add a note
fuku add --title "Database connection error"

# Search notes
fuku search "database"

# View with shortcuts
fuku view @latest    # Latest note
fuku view @1         # First search result
```

---

## Installation Section

### Download Options

#### Direct Downloads (Recommended)
**Latest Version**: v0.3.1 (auto-update from GitHub API)

**Platforms**:
- **Linux (x86_64)**: `fukura-x86_64-unknown-linux-gnu.tar.xz`
- **Linux (ARM64)**: `fukura-aarch64-unknown-linux-gnu.tar.xz`
- **macOS (Intel)**: `fukura-x86_64-apple-darwin.tar.xz`
- **macOS (Apple Silicon)**: `fukura-aarch64-apple-darwin.tar.xz`
- **Windows (x86_64)**: `fukura-x86_64-pc-windows-msvc.zip`

**Download Base URL**:
```
https://github.com/boostbit-inc/fukura/releases/latest/download/{filename}
```

#### Package Managers (Coming Soon)
- APT (Debian/Ubuntu) - In development
- Homebrew (macOS) - Planned
- Chocolatey (Windows) - Planned

#### Docker
```bash
docker pull ghcr.io/boostbit-inc/fukura:latest
docker run --rm -it ghcr.io/boostbit-inc/fukura:latest fuku --help
```

#### From Source
```bash
git clone https://github.com/boostbit-inc/fukura.git
cd fukura
cargo install --path .
```

---

## Commands Reference

### Core Commands

#### `fuku init`
Initialize a new repository
- Interactive setup for daemon and sync preferences
- Creates `.fukura/` directory in current project
- Optional daemon and hook installation

#### `fuku add`
Add a new note
- `--title`: Note title
- `--tag`: Add tags (multiple)
- `--stdin`: Read from stdin
- `--file`: Read from file
- `--privacy`: private (default) / org / public

#### `fuku search`
Search notes
- `-n, --limit`: Max results (default: 20)
- `-s, --sort`: Sort by (relevance/updated/likes)
- `-a, --all-repos`: Search all local repositories
- `--tui`: Interactive TUI mode
- `--json`: JSON output

#### `fuku view`
View a note
- Accepts: full ID, short ID, @latest, @1, @2, etc.
- `--json`: JSON output

#### `fuku open`
Open note in browser
- Beautiful HTML rendering
- `--theme`: light/dark
- `--url-only`: Show URL without opening

#### `fuku sync`
Sync notes with remote
- `<id>`: Sync specific note
- `--all`: Sync all notes
- `--enable-auto`: Enable auto-sync
- `--disable-auto`: Disable auto-sync

#### `fuku daemon`
Manage background daemon
- Default: Start daemon
- `--status`: Show detailed status
- `--stop`: Stop daemon
- `--foreground`: Run in foreground (debug)

#### `fuku hook`
Manage shell hooks
- `--install`: Install hooks
- `--uninstall`: Remove hooks
- `--status`: Check installation

#### `fuku gc`
Optimize storage
- Default: Pack objects
- `--prune`: Remove loose objects after packing

#### `fuku config`
Manage configuration
- `config remote --set <url>`: Set remote URL
- `config remote --global`: Apply globally
- `config redact --set name=pattern`: Add redaction rule

---

## Key Features for Site

### 1. Intelligent Shortcuts
```bash
fuku view @latest     # Most recent note
fuku view @1          # First search result
fuku open a664dd      # Short ID (8 chars)
```

### 2. Global Configuration
```bash
# Set once, use everywhere
fuku config remote --set https://hub.example.com --global
```

User-wide defaults stored in `~/.fukura/config.toml`, automatically inherited by all projects.

### 3. Privacy-First Design
- All notes **private by default**
- Stored locally in project's `.fukura/`
- Sync only when explicitly requested
- Auto-redaction of secrets before any sync

### 4. Enhanced Security Redaction
Automatically redacts:
- AWS credentials (access keys, secret keys)
- GitHub tokens (ghp_, gho_)
- API keys and bearer tokens
- Passwords and secrets
- JWT tokens
- Database connection strings
- Private keys (RSA, EC)
- IP addresses (optional)
- Email addresses

### 5. Git-Like Workflow
```bash
fuku init              # Like git init
fuku add               # Like git add
fuku search            # Find your content
fuku sync              # Like git push
```

---

## Installation Instructions (Detailed)

### Linux (Ubuntu/Debian/WSL2)
```bash
# Download
curl -LO https://github.com/boostbit-inc/fukura/releases/latest/download/fukura-x86_64-unknown-linux-gnu.tar.xz

# Extract
tar -xf fukura-x86_64-unknown-linux-gnu.tar.xz

# Install
sudo mv fukura /usr/local/bin/
sudo mv fuku /usr/local/bin/

# Verify
fuku --version
```

### macOS
```bash
# Download (Apple Silicon)
curl -LO https://github.com/boostbit-inc/fukura/releases/latest/download/fukura-aarch64-apple-darwin.tar.xz

# Or Intel Mac
curl -LO https://github.com/boostbit-inc/fukura/releases/latest/download/fukura-x86_64-apple-darwin.tar.xz

# Extract
tar -xf fukura-*.tar.xz

# Install
sudo mv fukura /usr/local/bin/
sudo mv fuku /usr/local/bin/

# Verify
fuku --version
```

### Windows
```powershell
# Download
curl -LO https://github.com/boostbit-inc/fukura/releases/latest/download/fukura-x86_64-pc-windows-msvc.zip

# Extract
Expand-Archive fukura-x86_64-pc-windows-msvc.zip -DestinationPath C:\Tools\fukura

# Add to PATH (PowerShell as Administrator)
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Tools\fukura", "Machine")

# Restart terminal, then verify
fuku --version
```

---

## FAQ Section

### Q: Is Fukura free and open source?
**A**: Yes! Fukura is 100% open source under Apache-2.0 license.

### Q: Does Fukura collect any data?
**A**: No. Zero telemetry, zero tracking. All data stays on your machine.

### Q: How does the daemon work?
**A**: The daemon runs in the background, monitoring command executions and errors through shell hooks. It's completely transparent - run `fuku daemon --status` to see exactly what it monitors and where data is stored.

### Q: Can I use Fukura with my team?
**A**: Yes! Use `fuku sync` to share notes with your team via FukuraHub (coming soon) or self-hosted instance.

### Q: What about sensitive data?
**A**: Fukura automatically redacts 14+ types of sensitive data (AWS keys, passwords, tokens, etc.) before any storage or sync. You can also add custom redaction patterns.

### Q: Does it work on Windows/WSL2?
**A**: Yes! Fukura supports Windows natively, plus Linux, macOS, and WSL2.

---

## Call-to-Actions

### Primary CTA
```
Download Fukura v0.3.1
```
Action: Auto-detect platform and show appropriate download link

### Secondary CTA
```
View Documentation
```
Action: Link to docs section or GitHub

### Tertiary CTA
```
View on GitHub
```
Action: https://github.com/boostbit-inc/fukura

---

## Version Info (Auto-Update)

**Latest Release**: v0.3.1
**Release Date**: October 10, 2025
**Download Count**: (fetch from GitHub API)

**Release Notes Highlights**:
- Git-level UX refinement
- Simplified command interface
- Enhanced security (14+ redaction patterns)
- Global configuration support
- Smart shortcuts (@latest, @1, short IDs)
- CI/CD compliance fixes

---

## Social Proof / Stats (if available)

- ⭐ GitHub Stars: (fetch from API)
- 📦 Downloads: (fetch from API)
- 🔄 Active Development: Yes
- 📄 License: Apache-2.0
- 🛡️ Security: Enterprise-grade

---

## Installation Video / GIF (Suggested)

Show terminal recording of:
```bash
$ fuku init
$ fuku add --title "Connection timeout"
$ fuku search "timeout"
$ fuku view @1
```

---

## Notes for fukura-site Implementation

1. **Auto-fetch latest version**: Use server-side API route to avoid CORS/firewall issues
2. **Platform detection**: Auto-detect user's OS and show relevant install command
3. **Copy buttons**: Add copy-to-clipboard for all code blocks
4. **Error handling**: Graceful fallback if GitHub API unavailable
5. **Update frequency**: Cache version info for 1 hour to reduce API calls

---

## API Endpoint Suggestion for fukura-site

```typescript
// pages/api/version.ts
export async function GET() {
  const response = await fetch(
    'https://api.github.com/repos/boostbit-inc/fukura/releases/latest',
    {
      headers: {
        'Accept': 'application/vnd.github.v3+json',
        'User-Agent': 'fukura-site'
      },
      next: { revalidate: 3600 } // Cache for 1 hour
    }
  );
  
  const data = await response.json();
  
  return Response.json({
    version: data.tag_name,
    published_at: data.published_at,
    download_count: data.assets.reduce((sum, a) => sum + a.download_count, 0),
    assets: data.assets.map(a => ({
      name: a.name,
      url: a.browser_download_url,
      size: a.size,
      platform: detectPlatform(a.name)
    }))
  });
}
```

---

このファイルをコピーしたら削除してください。



