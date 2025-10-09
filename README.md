# Fukura CLI

Fukura is a Rust CLI for capturing recurring error fixes in a content-addressable store, with fast search (Tantivy), redaction rules, pack files, and a TUI. This repository builds the CLI, packages platform installers, and produces assets for the `fukura.dev` download site.

## Install

### Debian / Ubuntu (recommended)

```bash
# 1) install the signing key
wget -qO - https://fukura.dev/apt/fukura-archive-keyring.gpg | sudo tee /usr/share/keyrings/fukura.gpg > /dev/null

# 2) register the repository
echo "deb [signed-by=/usr/share/keyrings/fukura.gpg] https://fukura.dev/apt stable main" | sudo tee /etc/apt/sources.list.d/fukura.list

# 3) install
sudo apt update
sudo apt install fukura
```

(If you host your own mirror, replace the URLs above with your endpoint. The CI job publishes a ready-to-serve `dist/apt` tree.)

### Portable binaries

Download the latest archive for your platform from GitHub Releases:

```bash
curl -LO https://github.com/boostbit-inc/fukura/releases/download/<tag>/fukura-<platform>.tar.gz
mkdir -p ~/.local/bin
cd ~/.local/bin
tar -xzf ~/Downloads/fukura-<platform>.tar.gz
export PATH="$HOME/.local/bin:$PATH"
```

### Build from source

```bash
git clone https://github.com/boostbit-inc/fukura.git
cd fukura
cargo install --path .
```

The binary installs as `fukura` and a convenience alias `fuku` is provided.

## 🚀 Quickstart

### Automatic Error Capture (Recommended)

```bash
fukura init                              # Interactive setup with daemon & sync options
# Now just develop normally - Fukura automatically captures errors and solutions!

# After 5 minutes of inactivity, sessions become auto-generated notes
fukura search "cargo build error"        # Find auto-generated solutions
fukura view <auto-note-id>               # View detailed error + solution notes
```

### Manual Usage (Traditional)

```bash
fukura init --no-daemon                  # Initialize without auto-daemon
fukura add --title "Proxy deploy"        # capture a note (stdin/editor/file)
fukura search "proxy timeout" --tui      # multi-pane TUI; Tab switches panes
fukura open <id>                         # render as HTML in your browser
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
# Reference notes by shortcuts
fuku view @latest                        # View the most recent note
fuku view @1                             # View first note from search results
fuku open @2                             # Open second note from search results

# Short ID support
fuku view a664dd                         # Use first 6-8 chars instead of full hash
fuku sync f2f85e                         # Works with all commands accepting IDs

# Global configuration (applies to all projects)
fuku config remote --set https://hub.example.com --global

# View all notes across all projects
fuku search "" --all-repos                # Search across all local repositories
```

**Important: Local vs Global**
- Each project has its own `.fukura/` directory with project-specific notes
- Global config (`~/.fukura/config.toml`) provides default values (remote URL, auto-sync)
- Notes are NOT stored globally - they remain in each project
- `.fukura/` should stay in `.gitignore` (already configured)

## Repository layout

```
.
├── src/                      # CLI implementation
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

- **Fast Search**: Tantivy-based full-text search
- **Efficient Storage**: Pack files reduce disk usage
- **Optimized Indexing**: Incremental indexing for large repositories
- **Low Memory**: Designed for resource-constrained environments

## Licenses

- Code: Apache-2.0
- Documentation: CC-BY 4.0

## Notes for maintainers

- `GITHUB_TOKEN` is automatically provided to GitHub Actions; a separate secret is unnecessary unless you need elevated permissions.
- Supply `LINUX_GPG_KEY` / `LINUX_GPG_PASSPHRASE` (base64-encoded private key + passphrase) so releases produce signed packages.
- When Windows/macOS certificates are available, re-enable the commented matrix entries and signing steps in `release.yml`.

First commit suggestion: `chore: bootstrap fukura cli` (or similar) once the initial scaffold is ready.
