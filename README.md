# Fukura CLI

Fukura is a Rust CLI for capturing recurring error fixes in a content-addressable store, with fast search (Tantivy), redaction rules, pack files, and a TUI. This repository builds the CLI, packages platform installers, and produces assets for the `fukura.dev` download site.

## Install

### Linux / macOS / WSL2

Download and extract the latest release for your platform:

```bash
# Download (replace with your platform)
curl -LO https://github.com/boostbit-inc/fukura/releases/latest/download/fukura-x86_64-unknown-linux-gnu.tar.xz

# Extract
tar -xf fukura-x86_64-unknown-linux-gnu.tar.xz

# Move to PATH
sudo mv fukura /usr/local/bin/
sudo mv fuku /usr/local/bin/

# Verify installation
fuku --version
```

**Platform URLs:**
- Linux (x86_64): `fukura-x86_64-unknown-linux-gnu.tar.xz`
- Linux (ARM64): `fukura-aarch64-unknown-linux-gnu.tar.xz`
- macOS (Intel): `fukura-x86_64-apple-darwin.tar.xz`
- macOS (Apple Silicon): `fukura-aarch64-apple-darwin.tar.xz`

### Windows

Download and extract the latest release:

```powershell
# Download
curl -LO https://github.com/boostbit-inc/fukura/releases/latest/download/fukura-x86_64-pc-windows-msvc.zip

# Extract to desired location
Expand-Archive fukura-x86_64-pc-windows-msvc.zip -DestinationPath C:\Tools\fukura

# Add to PATH (PowerShell as Administrator)
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Tools\fukura", "Machine")

# Verify installation
fuku --version
```

### From Source

```bash
git clone https://github.com/boostbit-inc/fukura.git
cd fukura
cargo install --path .
```

The binary installs as `fukura` and a convenience alias `fuku` is provided.

### APT Repository (Coming Soon)

APT repository hosting is planned for future releases. Currently, please use direct downloads from GitHub Releases.

## 🚀 Quickstart

### Super Fast Setup (New!)

```bash
fuku init                                # Interactive setup
fuku completions zsh                     # Enable Tab completion (or bash/fish)
fuku alias --setup                       # Install quick aliases
source ~/.zshrc                          # Reload shell

# Now use super-fast aliases:
fa                                       # Quick add (fuku add -q)
fl                                       # List all notes
fs "docker"                              # Search
fv @1                                    # View first result
fe @1 --add-tag urgent                   # Edit and tag
```

### Automatic Error Capture (Recommended)

```bash
fuku init                                # Interactive setup with daemon & sync options
# Now just develop normally - Fukura automatically captures errors and solutions!

# After 5 minutes of inactivity, sessions become auto-generated notes
fuku search "cargo build error"          # Find auto-generated solutions
fuku view @1                             # View first result from search
fuku open @1                             # Open in browser
```

### Manual Usage (Traditional)

```bash
fuku init --no-daemon                    # Initialize without auto-daemon
fuku add -q                              # Quick add with prompts
fuku add --title "Proxy deploy"          # Full add (stdin/editor/file)
fuku search "proxy timeout" --tui        # multi-pane TUI; Tab switches panes
fuku open @latest                        # render as HTML in your browser
```

### Syncing with Remote (Fukurahub)

Fukura provides an intuitive `sync` command for sharing knowledge:

```bash
# Enable auto-sync (notes automatically sync after creation)
fukura sync --enable-auto

# Set default remote
fukura config remote --set https://fukurahub.example.com

# Sync a specific note
fukura sync <note-id>

# Sync all private notes to your remote
fukura sync --all

# Disable auto-sync
fukura sync --disable-auto
```

**Privacy-First Workflow:**
- All notes are **Private** by default
- Stored locally in `.fukura/`
- Only synced when you explicitly run `fukura sync`
- Review and edit on Fukurahub before making public

### Daemon Management

```bash
fukura daemon --status                   # Check daemon status (detailed info)
fukura monitor --auto-start              # Auto-start daemon for current directory
fukura hook --install                    # Install shell hooks for error capture
```

**What are hooks?**
Shell hooks integrate Fukura into your shell (bash/zsh/fish/powershell) to automatically capture:
- Command executions and exit codes
- Error messages from stderr  
- Working directory and git context

This enables passive error capture without manual intervention.

**What is gc (garbage collection)?**
The `gc` command packs loose note objects into efficient pack files:
```bash
fukura gc              # Pack objects for better performance
fukura gc --prune      # Pack and remove loose objects
```
This optimizes storage and improves search performance, similar to `git gc`.

### Shortcuts and Conveniences

Fukura provides several shortcuts for improved usability:

```bash
# Quick aliases (after 'fuku alias --setup')
fa                                       # fuku add -q (quick add)
fl                                       # fuku list
fs "query"                               # fuku search "query"
fv @1                                    # fuku view @1
fe @1 --add-tag urgent                   # fuku edit @1 --add-tag urgent

# Reference notes by shortcuts
fuku view @latest                        # View the most recent note
fuku view @1                             # View first note from last search results
fuku open @2                             # Open second note from search results

# Short ID support (8 chars instead of 64)
fuku view a664dd                         # Use first 6-8 chars instead of full hash
fuku sync f2f85e                         # Works with all commands accepting IDs

# Quick commands
fuku list                                # List all notes (alias for search "")
fuku stats                               # Show repository statistics
fuku config show                         # Display current configuration
fuku edit @latest --add-tag fix          # Edit and tag latest note

# Batch operations
fuku import ./old-notes/ --tag imported  # Import markdown files in bulk

# Global configuration (applies to all projects)
fuku config remote --set https://hub.example.com --global

# View all notes across all projects
fuku search "" --all-repos               # Search across all local repositories
```

**Important: Local vs Global**
- Each project has its own `.fukura/` directory with project-specific notes
- Global config (`~/.fukura/config.toml`) provides default values (remote URL, auto-sync)
- Notes are NOT stored globally - they remain in each project
- `.fukura/` should stay in `.gitignore` (already configured)

## ✨ New Features (v0.3.5+)

### Shell Completions (Tab Completion!)
```bash
# Install completions for your shell
fuku completions bash    # Bash
fuku completions zsh     # Zsh
fuku completions fish    # Fish
fuku completions powershell --stdout >> $PROFILE  # PowerShell

# After installation, Tab key autocompletes:
fuku v<Tab>              # → view, completes to 'fuku view'
fuku view @<Tab>         # → @latest, @1, @2...
fuku edit @1 --add-<Tab> # → --add-tag
```

### Quick Aliases
```bash
# Setup convenient aliases
fuku alias --setup

# Use super-fast shortcuts
fa       # Quick add with prompts
fl       # List all notes
fs       # Search
fv       # View
fe       # Edit
fo       # Open in browser
fst      # Stats
fsy      # Sync

# Example workflow
fa                          # Add note interactively
fs docker                   # Search
fv @1                       # View first result
fe @1 --add-tag production  # Edit and tag
```

### Batch Import
```bash
# Import existing markdown files
fuku import ./my-old-notes/          # Import directory
fuku import ./single-note.md         # Import single file
fuku import ./docs/ --tag imported   # Add default tag
fuku import ./work/ --dry-run        # Preview before importing
```

### Enhanced Commands
```bash
# New commands
fuku list                    # List all notes (cleaner than search "")
fuku stats                   # Repository statistics
fuku config show             # View all configuration
fuku edit @1 --add-tag fix   # Edit notes and manage tags

# Improved features
fuku add -q                  # Quick mode with interactive prompts
fuku add -t "Title" -b "Body" # Short flags
fuku sync                    # Syncs all notes by default (no --all needed)
```

## Repository layout

```
.
├── src/
│   ├── ui/                   # CLI/TUI entry points and presentation
│   ├── application/          # Use-case orchestrators and daemon control
│   ├── domain/               # Core models and business rules
│   ├── infrastructure/       # Storage, search, sync, OS integrations
│   └── shared/               # Cross-cutting utilities
├── tests/                    # integration tests
├── installers/               # WiX template, macOS postinstall, Linux postinst
├── scripts/linux/build-apt-repo.sh  # helper to stage an APT repo
├── dist-workspace.toml       # cargo-dist workspace configuration
└── .github/workflows/        # release + site dispatch workflows
```

## Release automation

- `cargo-dist` builds portable archives, `.deb`/`.rpm`, and staging data for the APT mirror.
- CI (`.github/workflows/release.yml`) currently targets Linux. Windows/macOS publishing is disabled until signing certificates are available; uncomment the matrix entries when ready.
- The release job:
  1. runs tests,
  2. `cargo dist build` for Linux targets,
  3. signs `.deb`/`.rpm` if `LINUX_GPG_KEY`/`LINUX_GPG_PASSPHRASE` are present,
  4. runs `scripts/linux/build-apt-repo.sh` to produce `dist/apt`,
  5. uploads artifacts and publishes the GitHub Release via `cargo dist upload` (uses the built-in `GITHUB_TOKEN`).
- `.github/workflows/site-deploy.yml` optionally pings `boostbit-inc/fukura-site` to redeploy `fukura.dev`.

### 🚀 What happens when you release?

When you create a release (by pushing a tag like `v0.2.0`), the following happens automatically:

1. **GitHub Release Creation**
   - Binary packages (`.deb`, `.rpm`, `.tar.gz`) are generated
   - Release notes are published on GitHub Releases page
   - Users can download packages directly from GitHub

2. **APT Repository Update**
   - Debian/Ubuntu users can install with: `sudo apt install fukura`
   - Package signing ensures authenticity and integrity
   - Automatic updates through system package manager

3. **Docker Image Update**
   - `ghcr.io/boostbit-inc/fukura:latest` is updated
   - Users can run: `docker pull ghcr.io/boostbit-inc/fukura`
   - Multi-platform support (Linux AMD64/ARM64)

4. **Website Deployment**
   - `fukura.dev` download page is automatically updated
   - New version information is published
   - Installation instructions reflect latest release

### 📊 Performance Optimizations

Recent improvements include:
- **Batch Processing**: Multiple notes are processed together for better performance
- **Memory Optimization**: Reduced memory allocations in pack processing
- **Search Performance**: Unstable sorting for large result sets
- **Daemon Efficiency**: Optimized session cleanup and directory monitoring

**Benchmark Results (50 notes):**
- Search: ~1.7ms average
- Load note: ~42µs average
- Store note: ~699ms (with fsync for data safety)

Run benchmarks yourself:
```bash
cargo bench
```

## 🛠️ Development

```bash
# Format code
cargo fmt

# Lint code
cargo clippy --all-targets --all-features

# Run all tests (including GitHub Actions tests)
cargo test

# Run specific test suites
cargo test --test github_actions
cargo test --test integration
cargo test --test performance

# Security audit
cargo audit

# License check
cargo deny check
```

To build release artifacts locally:

```bash
cargo install cargo-dist --locked
cargo dist build --target x86_64-unknown-linux-gnu --artifacts all
scripts/linux/build-apt-repo.sh dist
```

## Security

Fukura takes security seriously:

- **Automatic Secret Redaction**: AWS keys, API tokens, passwords, and more are automatically redacted
- **Privacy-First**: All notes are private by default, stored locally
- **Customizable Patterns**: Add organization-specific redaction rules
- **No Telemetry**: Zero data collection or tracking

See [docs/security.md](docs/security.md) for detailed security information.

## Performance

- **Fast Search**: Tantivy-based full-text search (~1.7ms for 50 notes)
- **Efficient Storage**: Pack files reduce disk usage
- **Optimized Indexing**: Incremental indexing for large repositories
- **Low Memory**: Designed for resource-constrained environments
- **Quick Load**: Note loading in ~42µs

## 🎨 GUI Overlay: Technical Feasibility

### Should We Add a GUI Overlay?

**Current Implementation:**
- ✅ OS-native notifications (macOS/Linux/Windows)
- ✅ Error detection and solution suggestions
- ✅ Beautiful HTML rendering in browser
- ✅ Interactive TUI for search

**Option: Rich GUI Overlay (Floating Window)**

**How It Works:**
1. **Using `egui`**: Immediate mode GUI framework
   - Pure Rust, cross-platform
   - Renders GUI in a separate window
   - ~500-1000 lines of code
   - Lightweight (~2MB binary increase)

2. **Using `tauri`**: Web-based GUI
   - HTML/CSS/JS frontend + Rust backend
   - Native window with WebView
   - ~2000-3000 lines of code
   - Larger binary (~5-10MB increase)

**Recommendation: Stick with Current Approach**

**Why?**
- ✅ Terminal-first design matches developer workflow
- ✅ Notifications work great for passive monitoring
- ✅ Browser rendering for detailed viewing
- ✅ TUI for interactive search
- ❌ GUI overlay would be intrusive
- ❌ Adds complexity and maintenance burden
- ❌ Not aligned with CLI philosophy

**If You Really Want GUI:**
- Use `fuku serve` + browser dashboard
- Keep it as separate web UI project
- Don't clutter the CLI with GUI dependencies

## Licenses

- Code: Apache-2.0
- Documentation: CC-BY 4.0

## Notes for maintainers

- `GITHUB_TOKEN` is automatically provided to GitHub Actions; a separate secret is unnecessary unless you need elevated permissions.
- Supply `LINUX_GPG_KEY` / `LINUX_GPG_PASSPHRASE` (base64-encoded private key + passphrase) so releases produce signed packages.
- When Windows/macOS certificates are available, re-enable the commented matrix entries and signing steps in `release.yml`.

First commit suggestion: `chore: bootstrap fukura cli` (or similar) once the initial scaffold is ready.
