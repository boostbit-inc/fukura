# Fukura Command Reference

Complete reference for all `fuku` commands with detailed options and examples.

## Table of Contents

- [Installation](#installation)
- [Global Options](#global-options)
- [Commands](#commands)
  - [init](#init)
  - [add](#add)
  - [search](#search)
  - [view](#view)
  - [open](#open)
  - [serve](#serve)
  - [gc](#gc)
  - [push](#push)
  - [pull](#pull)
  - [sync](#sync)
  - [config](#config)
  - [daemon](#daemon)

---

## Installation

### APT (Debian/Ubuntu)

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

### One-line Install Script

```bash
curl -sSL https://fukura.dev/install.sh | bash
```

### Binary Download

Download the latest release for your platform from [GitHub Releases](https://github.com/boostbit-inc/fukura/releases).

---

## Global Options

These options can be used with any command:

```bash
fuku [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--repo <PATH>` | Path to the repository root (defaults to current directory) |
| `--quiet` | Suppress celebratory output |
| `--version` | Display version information |
| `--help` | Display help information |

---

## Commands

### init

Initialize a new Fukura repository in a directory.

**Usage:**
```bash
fuku init [PATH] [OPTIONS]
```

**Arguments:**

| Argument | Description | Default |
|----------|-------------|---------|
| `PATH` | Directory to initialize | `.` (current directory) |

**Options:**

| Option | Description |
|--------|-------------|
| `--force` | Reinitialize existing repository |
| `--no-daemon` | Skip daemon setup |
| `--no-hooks` | Skip shell hooks installation |

**Examples:**

```bash
# Initialize in current directory
fuku init

# Initialize in specific directory
fuku init ~/projects/myapp

# Initialize without daemon
fuku init --no-daemon

# Reinitialize existing repository
fuku init --force
```

**What it does:**
1. Creates `.fukura` directory structure
2. Sets up configuration files
3. Initializes search index
4. Optionally starts error capture daemon
5. Installs shell hooks for automatic error detection

---

### add

Add a new note to the repository.

**Usage:**
```bash
fuku add [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--title <TEXT>` | Note title |
| `--body <TEXT>` | Note content |
| `--file <PATH>` | Read content from file |
| `--stdin` | Read content from stdin |
| `--tag <TAG>` | Add tag (can be used multiple times) |
| `--meta <KEY=VALUE>` | Add metadata (can be used multiple times) |
| `--link <URL>` | Add link (can be used multiple times) |
| `--privacy <PRIVACY>` | Privacy level: `private`, `org`, or `public` (default: `private`) |
| `--author <NAME>` | Author name |
| `--email <EMAIL>` | Author email |
| `--no-editor` | Skip editor and use inline input |

**Examples:**

```bash
# Interactive mode (opens editor)
fuku add --title "Fixed CORS issue"

# With all options inline
fuku add \
  --title "Redis connection error" \
  --body "Added retry logic with exponential backoff" \
  --tag redis \
  --tag error-handling \
  --meta severity=high \
  --link "https://redis.io/topics/connection-handling" \
  --privacy org

# From file
fuku add --title "Deploy script" --file ./deploy.sh

# From stdin
echo "Quick note" | fuku add --title "Temp fix" --stdin

# Pipe command output
git log --oneline -5 | fuku add --title "Recent commits" --stdin

# No editor (inline input)
fuku add --title "Quick note" --no-editor
```

---

### search

Search notes in the repository.

**Usage:**
```bash
fuku search [QUERY] [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `QUERY` | Search terms (multiple words supported) |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-n, --limit <N>` | Maximum number of results | 20 |
| `-s, --sort <SORT>` | Sort by: `relevance`, `updated`, or `likes` | `relevance` |
| `--json` | Output as JSON |
| `--tui` | Interactive TUI mode |
| `-a, --all-repos` | Search across all local repositories |

**Examples:**

```bash
# Basic search
fuku search "database error"

# Limit results
fuku search "api" --limit 5

# Sort by most recent
fuku search "deployment" --sort updated

# Search with multiple terms
fuku search cors nginx api

# Interactive TUI mode
fuku search "redis" --tui

# Search all repositories
fuku search "kubernetes" --all-repos

# JSON output for scripting
fuku search "error" --json | jq '.[] | .title'
```

**TUI Mode Controls:**

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate results |
| `Tab` | Switch between results and filters |
| `Space` | Toggle tag filter |
| `1-4` | Set time filter (All/7d/30d/90d) |
| `f` | Clear all filters |
| `o` | Open note in browser |
| `r` | Refresh search |
| `q/Esc` | Exit TUI |

---

### view

View a note's contents.

**Usage:**
```bash
fuku view <ID> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `ID` | Note ID (full or short), or special refs: `@latest`, `@1`, `@2`, etc. |

**Options:**

| Option | Description |
|--------|-------------|
| `--json` | Output as JSON |

**Examples:**

```bash
# View by full ID
fuku view a3f8e9b2c4d5e6f7

# View by short ID (first 8 chars)
fuku view a3f8e9b2

# View latest note
fuku view @latest

# View second most recent note
fuku view @1

# JSON output
fuku view @latest --json
```

---

### open

Open a note in your web browser with rendered HTML.

**Usage:**
```bash
fuku open <ID> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `ID` | Note ID or special refs (`@latest`, `@1`, etc.) |

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--theme <THEME>` | Theme: `light` or `dark` | `dark` |
| `--browser-only` | Open in browser directly (no local server) |
| `--url-only` | Show URL only (don't open browser) |
| `--server-port <PORT>` | Local server port | `8080` |

**Examples:**

```bash
# Open latest note
fuku open @latest

# Open with light theme
fuku open a3f8e9b2 --theme light

# Direct browser opening (no server)
fuku open @latest --browser-only

# Just show the URL
fuku open a3f8e9b2 --url-only

# Custom server port
fuku open @latest --server-port 3000
```

---

### serve

Start a local web server to browse notes via HTTP API.

**Usage:**
```bash
fuku serve [OPTIONS]
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--addr <HOST:PORT>` | Server address | `127.0.0.1:8765` |
| `--page-size <N>` | Default page size | `50` |

**Examples:**

```bash
# Start with defaults
fuku serve

# Custom address and port
fuku serve --addr 0.0.0.0:8080

# Custom page size
fuku serve --page-size 100
```

**API Endpoints:**

```bash
# Health check
GET /healthz

# List/search notes
GET /notes?q=<query>&limit=<n>&sort=<relevance|updated|likes>

# Get specific note
GET /notes/:id

# Create note
POST /notes
```

---

### gc

Garbage collection: optimize storage by packing loose objects.

**Usage:**
```bash
fuku gc [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--prune` | Remove loose objects after packing |

**Examples:**

```bash
# Pack loose objects
fuku gc

# Pack and prune
fuku gc --prune
```

---

### push

Push a note to remote server.

**Usage:**
```bash
fuku push <ID> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `ID` | Note ID to push |

**Options:**

| Option | Description |
|--------|-------------|
| `--remote <URL>` | Remote URL (overrides config) |

**Examples:**

```bash
# Push with configured remote
fuku push a3f8e9b2

# Push to specific remote
fuku push a3f8e9b2 --remote https://hub.fukura.dev

# Push latest note
fuku push @latest
```

---

### pull

Pull a note from remote server.

**Usage:**
```bash
fuku pull <ID> [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `ID` | Remote note ID to pull |

**Options:**

| Option | Description |
|--------|-------------|
| `--remote <URL>` | Remote URL (overrides config) |

**Examples:**

```bash
# Pull from configured remote
fuku pull hub-id-12345

# Pull from specific remote
fuku pull hub-id-12345 --remote https://hub.fukura.dev
```

---

### sync

Sync notes with remote server.

**Usage:**
```bash
fuku sync [ID] [OPTIONS]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `ID` | Note ID to sync (optional with `--all`) |

**Options:**

| Option | Description |
|--------|-------------|
| `--remote <URL>` | Remote URL (overrides config) |
| `--all` | Sync all private notes |
| `--enable-auto` | Enable automatic syncing |
| `--disable-auto` | Disable automatic syncing |

**Examples:**

```bash
# Enable auto-sync
fuku sync --enable-auto

# Disable auto-sync
fuku sync --disable-auto

# Sync specific note
fuku sync a3f8e9b2

# Sync all private notes
fuku sync --all

# Sync to specific remote
fuku sync --all --remote https://hub.fukura.dev
```

---

### config

Manage configuration settings.

**Usage:**
```bash
fuku config <SUBCOMMAND>
```

**Subcommands:**

#### config remote

Configure remote URL for syncing.

**Usage:**
```bash
fuku config remote [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--set <URL>` | Set remote URL |
| `--clear` | Clear remote URL |
| `--global` | Apply globally (all projects) |

**Examples:**

```bash
# Set remote for current repository
fuku config remote --set https://hub.fukura.dev

# Set global remote (all repositories)
fuku config remote --set https://hub.fukura.dev --global

# Clear remote
fuku config remote --clear

# Clear global remote
fuku config remote --clear --global
```

#### config redact

Manage redaction rules for sensitive data.

**Usage:**
```bash
fuku config redact [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--set <NAME=REGEX>` | Add redaction rule (can be used multiple times) |
| `--unset <NAME>` | Remove redaction rule (can be used multiple times) |

**Examples:**

```bash
# Add redaction rule for API keys
fuku config redact --set api_key='sk-[a-zA-Z0-9]{32}'

# Add multiple rules
fuku config redact \
  --set api_key='sk-[a-zA-Z0-9]{32}' \
  --set password='password=\S+' \
  --set token='token:\s*\S+'

# Remove rule
fuku config redact --unset api_key
```

---

### daemon

Manage the background daemon for automatic error capture.

**Usage:**
```bash
fuku daemon [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--status` | Show daemon status and information |
| `--stop` | Stop the daemon |
| `--foreground` | Run daemon in foreground (for debugging) |
| `--install-hooks` | Install shell hooks |
| `--uninstall-hooks` | Uninstall shell hooks |
| `--hooks-status` | Check shell hooks status |
| `--notifications-enable` | Enable error notifications |
| `--notifications-disable` | Disable error notifications |
| `--notifications-status` | Check notification status |
| `--test-notification` | Test notifications (send test notification) |

**Examples:**

```bash
# Start daemon (default)
fuku daemon

# Check status
fuku daemon --status

# Stop daemon
fuku daemon --stop

# Run in foreground (debug mode)
fuku daemon --foreground

# Install shell hooks
fuku daemon --install-hooks

# Check hooks status
fuku daemon --hooks-status

# Enable notifications
fuku daemon --notifications-enable

# Test notifications
fuku daemon --test-notification
```

**What the daemon does:**

1. **Monitors shell sessions** - Tracks command executions via Unix Domain Socket (Unix) or Named Pipe (Windows)
2. **Captures errors** - Detects non-zero exit codes immediately
3. **Creates notes** - Auto-generates notes from errors instantly (no 5-minute wait)
4. **Provides context** - Records working directory, git branch, environment
5. **Sends notifications** - OS-native notifications with error details and past solutions
6. **Respects privacy** - All data stored locally unless explicitly synced

**Notification Features:**

- **Immediate** - Shows notification as soon as error occurs
- **Intelligent** - Searches past solutions and includes them in notification
- **Non-intrusive** - Does NOT auto-open browser
- **Detailed** - Shows command, error, and how to view details
- **Cross-platform** - macOS (osascript), Linux (D-Bus), Windows (Toast)

**Example Notification:**
```
Fukura: Error Captured (1 solution found)

Command: npm install nonexistent-pkg

You've solved this before:
  • Install the correct package name: npm install correct-package-name

View:
  fuku view 812639fa
  fuku open 812639fa
```

---

## Common Workflows

### Quick Start

```bash
# 1. Initialize repository
cd ~/projects/myapp
fuku init

# 2. Daemon automatically captures errors
npm test  # If this fails, Fukura captures it

# 3. View captured errors
fuku search error
fuku view @latest

# 4. Add solution
fuku add --title "Fixed npm test issue" --body "Solution: ..." --tag solved
```

### Automatic Error Capture

```bash
# The daemon captures errors automatically
cd ~/projects/myapp
fuku init  # Enables daemon by default

# Run commands normally - errors are auto-captured
npm test
cargo build
docker-compose up

# Instant access to errors
fuku search ""       # List all notes
fuku view @latest    # View most recent
fuku open @latest    # Open in browser
```

### Team Knowledge Sharing

```bash
# 1. Configure remote
fuku config remote --set https://hub.fukura.dev

# 2. Enable auto-sync
fuku sync --enable-auto

# 3. Add notes with org privacy
fuku add --title "Deployment guide" --privacy org

# 4. Sync all team notes
fuku sync --all

# 5. Pull teammate's note
fuku pull hub-id-12345
```

### Sensitive Data Protection

```bash
# Set up redaction rules
fuku config redact \
  --set api_key='sk-[a-zA-Z0-9]{32}' \
  --set password='password=\S+' \
  --set aws_key='AWS[A-Z0-9]{16}'

# Now all notes automatically redact sensitive patterns
fuku add --title "API integration" --file ./script.sh
```

---

## Differences Between Similar Commands

### sync vs push

| Command | Purpose | Use Case |
|---------|---------|----------|
| `fuku sync` | **Bidirectional sync** + auto-sync management | Team collaboration, enable auto-sync |
| `fuku push` | **One-way upload** only | Send specific note once |

### view vs search --tui

| Command | Purpose | Use Case |
|---------|---------|----------|
| `fuku view <id>` | View specific note (ID required) | Know exact note ID |
| `fuku search --tui` | Interactive search → select → preview | Don't know ID, want to explore |

---

## Configuration Files

### Local Config

`.fukura/config.toml` (per-repository)

```toml
[repository]
default_remote = "https://hub.fukura.dev"
auto_sync = true
daemon_enabled = true

[redaction]
api_key = "sk-[a-zA-Z0-9]{32}"
password = "password=\\S+"
```

### Notification Config

`.fukura/notification.toml` (per-repository)

```toml
enabled = true
show_on_error = true
show_on_solution_found = true
```

### Global Config

`~/.config/fukura/config.toml` (all repositories)

```toml
[repository]
default_remote = "https://hub.fukura.dev"

[user]
name = "John Doe"
email = "john@example.com"
```

---

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `FUKURA_REMOTE` | Default remote URL | `https://hub.fukura.dev` |
| `FUKURA_AUTO_SYNC` | Enable auto-sync | `true` |
| `GIT_AUTHOR_NAME` | Default author name | `John Doe` |
| `GIT_AUTHOR_EMAIL` | Default author email | `john@example.com` |
| `EDITOR` | Preferred text editor | `vim`, `nano`, `code` |
| `RUST_LOG` | Logging level | `debug`, `info`, `warn`, `error` |

---

## Troubleshooting

### Daemon won't start

```bash
# Check if already running
fuku daemon --status

# Stop and restart
fuku daemon --stop
fuku daemon

# Run in foreground to see errors
fuku daemon --foreground
```

### Notifications not showing

```bash
# Check notification status
fuku daemon --notifications-status

# Enable if disabled
fuku daemon --notifications-enable

# Test notifications
fuku daemon --test-notification

# Check if daemon is running
fuku daemon --status
```

### Search not finding notes

```bash
# Rebuild search index
rm -rf .fukura/index
fuku search ""  # Rebuilds index automatically
```

### Sync failing

```bash
# Check remote configuration
fuku config remote

# Test with explicit remote
fuku push @latest --remote https://hub.fukura.dev
```

---

## Tips and Tricks

### Alias Commands

```bash
# Add to ~/.bashrc or ~/.zshrc
alias fn='fuku add --title'
alias fs='fuku search'
alias fv='fuku view @latest'
alias fo='fuku open @latest'
```

Usage:
```bash
fn "Quick fix for bug #123"
fs "kubernetes"
fv
fo
```

### Pipe Output

```bash
# Capture command output
docker ps --all | fuku add --title "Container status" --stdin

# Capture logs
journalctl -u nginx -n 50 | fuku add --title "Nginx logs" --stdin

# Capture error output
npm test 2>&1 | fuku add --title "Test failures" --stdin
```

### JSON Processing

```bash
# Extract titles
fuku search "api" --json | jq -r '.[].title'

# Filter by tags
fuku search "" --json | jq '.[] | select(.tags | contains(["redis"]))'

# Count notes by tag
fuku search "" --json | jq '[.[].tags[]] | group_by(.) | map({tag: .[0], count: length})'
```

---

## Support

- Documentation: https://fukura.dev/docs
- GitHub: https://github.com/boostbit-inc/fukura
- Issues: https://github.com/boostbit-inc/fukura/issues
