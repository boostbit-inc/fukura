# Getting Started with Fukura

Welcome to Fukura! This guide will help you get up and running with Fukura's intelligent error capture and knowledge management system.

## 🚀 Installation

### Option 1: Install from Source

```bash
git clone https://github.com/boostbit-inc/fukura.git
cd fukura
cargo build --release
```

### Option 2: Install via Cargo

```bash
cargo install fukura
```

### Option 3: Using Docker

```bash
docker run -v $(pwd):/workspace ghcr.io/boostbit-inc/fukura:latest init
```

## 🎯 Automatic Error Capture (Recommended)

### 1. Initialize with Automatic Daemon

```bash
fuku init
```

This command:
- Creates a `.fukura/` directory
- Starts the background daemon automatically
- Installs shell hooks for error capture
- Begins monitoring your development environment

### 2. Develop Normally

Just work on your projects as usual! Fukura automatically captures:

- **Commands**: All shell commands with exit codes
- **Errors**: Stderr output and error messages  
- **Context**: Git branch, working directory, environment
- **Solutions**: Successful commands that resolved issues

### 3. View Auto-Generated Notes

After 5 minutes of inactivity, sessions automatically become notes:

```bash
# Search for auto-generated solutions
fuku search "cargo build error"

# View specific auto-generated notes
fuku view <auto-note-id>

# Start web server for better UI
fuku serve
```

### 4. Performance Benefits

Fukura includes several performance optimizations:

- **Batch Processing**: Multiple notes are processed together for faster operations
- **Memory Optimization**: Efficient memory usage during bulk operations
- **Fast Search**: Optimized search with Tantivy full-text indexing
- **Smart Caching**: Reduced I/O operations for better responsiveness

## 🔧 Manual Usage (Traditional)

If you prefer manual control:

### 1. Initialize Without Auto-Daemon

```bash
fuku init --no-daemon
```

### 2. Add Notes Manually

```bash
fuku add --title "My First Note"
```

You'll be prompted to enter the note body, or you can use flags:

```bash
fuku add --title "Rust Error Fix" --body "How to fix this Rust error" --tags rust,error,fix
```

### 3. Search and View

```bash
fuku search "rust error"
fuku view <note-id>
```

## 🎛️ Daemon Management

### Check Daemon Status

```bash
fuku daemon --status
```

### Manual Daemon Control

```bash
# Start daemon in foreground
fuku daemon --foreground

# Auto-start daemon for current directory
fuku monitor --auto-start

# Check monitoring status
fuku monitor --status
```

### Shell Integration

```bash
# Install shell hooks
fuku hook install

# Check hook status
fuku hook status

# Remove hooks
fuku hook uninstall
```

## 📊 Understanding Auto-Generated Notes

Auto-generated notes have this structure:

```markdown
# Auto-Captured: cargo build error

**Session Duration:** 120 seconds
**Working Directory:** /home/user/my-project
**Git Branch:** feature/new-feature

## Errors Encountered
- **stderr**: error[E0433]: failed to resolve: use of undeclared type

## Solution Steps
1. `cargo clean`
2. `cargo build`

## All Commands
1. ❌ `cargo build`
2. ✅ `cargo clean`
3. ✅ `cargo build`

Tags: [auto-captured, rust, cargo, build]
```

## 🔍 Advanced Features

### Tags

Auto-generated notes include intelligent tags:
- Technology tags: `rust`, `docker`, `git`, `python`
- Error type tags: `permissions`, `network`, `memory`
- Context tags: `auto-captured`, `session`

### Links

Link notes together:

```bash
fuku add --title "Related Note" --body "This links to the Rust error fix" --links <note-id>
```

### Privacy

Set note privacy levels:

```bash
fuku add --title "Private Note" --body "Sensitive information" --privacy private
```

### Browser Integration

Open notes in your browser:

```bash
fuku open <note-id>
```

Works cross-platform with automatic browser detection.

## 🎯 Best Practices

### For Automatic Capture

1. **Initialize once per project**: Run `fuku init` in your project root
2. **Work normally**: Just develop - Fukura captures everything automatically
3. **Review periodically**: Check `fuku search` to see captured solutions
4. **Tag manually**: Add custom tags to important auto-generated notes

### For Manual Usage

1. **Be descriptive**: Use clear titles and detailed bodies
2. **Use tags**: Organize with relevant tags
3. **Link related notes**: Create knowledge networks
4. **Regular sync**: Use `fuku push` to backup important notes

## 🚨 Troubleshooting

### Daemon Issues

```bash
# Check if daemon is running
fuku daemon --status

# Restart daemon
fuku daemon --stop
fuku daemon --foreground

# Check shell hooks
fuku hook status
```

### Permission Issues

```bash
# Make sure .fukura directory is writable
chmod -R 755 .fukura/

# Check file permissions
ls -la .fukura/
```

## 📚 Next Steps

- Explore the [Browser Integration](browser-integration.md) guide
- Check out the [Architecture Overview](../developer/architecture.md)
- Learn about [Contributing](../CONTRIBUTING.md)
- Read the [Security Policy](../SECURITY.md)