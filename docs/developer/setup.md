# Development Setup

This guide will help you set up a development environment for contributing to Fukura.

## Prerequisites

- **Rust**: Latest stable version (1.70+)
- **Git**: For version control
- **Docker**: For consistent build environments (optional but recommended)
- **Editor**: VS Code with rust-analyzer extension (recommended)

## Quick Setup

### 1. Clone the Repository
```bash
git clone https://github.com/boostbit-inc/fukura.git
cd fukura
```

### 2. Install Dependencies
```bash
# Install Rust toolchain
rustup update

# Install development tools
cargo install cargo-deny cargo-audit cargo-outdated
```

### 3. Build the Project
```bash
cargo build
```

### 4. Run Tests
```bash
cargo test
```

## Development Environment

### Using Docker (Recommended)
```bash
# Build development container
docker-compose up -d

# Run commands in container
docker-compose exec fukura cargo build
docker-compose exec fukura cargo test
```

### Local Development
```bash
# Install dependencies
cargo build

# Run tests
cargo test

# Run benchmarks
cargo bench

# Check code quality
cargo clippy
cargo fmt
```

## Code Quality Tools

### Pre-commit Setup
```bash
# Install pre-commit hooks
cargo install cargo-husky
cargo husky install
```

### Running Quality Checks
```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Security audit
cargo audit

# License compliance
cargo deny check

# Check for outdated dependencies
cargo outdated
```

### Performance Testing
```bash
# Run performance benchmarks
cargo bench

# Run memory usage tests
cargo test --test performance

# Check for performance regressions
cargo test --test performance --release
```

### GitHub Actions Testing
```bash
# Test GitHub Actions configuration
cargo test --test github_actions

# Verify workflow files are valid
# (GitHub Actions validates YAML syntax automatically)
```

## Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
cargo test --test integration
```

### Performance Tests
```bash
cargo test --test performance
```

### Security Tests
```bash
cargo test --test security
```

### Benchmarks
```bash
cargo bench
```

## Project Structure

```
fukura/
├── src/                    # Source code
│   ├── bin/               # Binary entry points
│   ├── cli.rs             # CLI interface
│   ├── index.rs           # Search indexing
│   ├── models.rs          # Data models
│   ├── repo.rs            # Repository management
│   └── ...
├── tests/                 # Integration tests
├── benches/               # Performance benchmarks
├── docs/                  # Documentation
├── .github/workflows/     # CI/CD workflows
├── Dockerfile             # Docker configuration
└── Cargo.toml            # Project configuration
```

## Development Workflow

### 1. Create a Feature Branch
```bash
git checkout -b feature/your-feature-name
```

### 2. Make Changes
- Write code following Rust conventions
- Add tests for new functionality
- Update documentation as needed

### 3. Run Quality Checks
```bash
cargo fmt
cargo clippy
cargo test
cargo audit
```

### 4. Commit Changes
```bash
git add .
git commit -m "feat: your feature description"
```

### 5. Push and Create PR
```bash
git push origin feature/your-feature-name
```

## Debugging

### Using Debug Builds
```bash
cargo build
RUST_LOG=debug ./target/debug/fukura --help
```

### Using GDB
```bash
cargo build
gdb ./target/debug/fukura
```

### Using VS Code Debugger
1. Install the "CodeLLDB" extension
2. Create `.vscode/launch.json`:
```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug fukura",
            "cargo": {
                "args": ["build", "--bin=fukura"],
                "filter": {
                    "name": "fukura",
                    "kind": "bin"
                }
            },
            "args": ["--help"],
            "cwd": "${workspaceFolder}"
        }
    ]
}
```

## Performance Profiling

### Using Cargo Instruments (macOS)
```bash
cargo install cargo-instruments
cargo instruments -t "Time Profiler" --bin fukura
```

### Using Flamegraph
```bash
cargo install flamegraph
cargo flamegraph --bin fukura
```

## Cross-Compilation

### Setup Cross-Compilation
```bash
# Add target
rustup target add x86_64-unknown-linux-gnu

# Install cross-compilation tools
cargo install cross
```

### Build for Different Targets
```bash
# Linux
cross build --target x86_64-unknown-linux-gnu --release

# Windows
cross build --target x86_64-pc-windows-gnu --release
```

## Troubleshooting

### Common Issues

1. **Build Fails with OpenSSL Errors**
   - Solution: Use Docker or ensure OpenSSL development libraries are installed

2. **Tests Fail on Windows**
   - Solution: Use WSL2 or Docker for consistent behavior

3. **Performance Tests Timeout**
   - Solution: Increase timeout or run on more powerful hardware

### Getting Help

- Check existing issues on GitHub
- Ask questions in discussions
- Join our Discord community (if available)

## Release Process

Before creating a release, follow this checklist to ensure quality:

### Pre-Release Checklist

```bash
# 1. Format code
cargo fmt --all

# 2. Check formatting (CI requirement)
cargo fmt --all -- --check

# 3. Lint with strict warnings
cargo clippy --all-targets --all-features -- -D warnings

# 4. Run full test suite
cargo test --all

# 5. Build release version
cargo build --release

# 6. Verify installation
cargo install --path . --force
fuku --version
```

### Release Steps

1. **Update version** in `Cargo.toml`
2. **Update Cargo.lock**: `cargo update -p fukura`
3. **Commit**: `git commit -am "chore: release vX.Y.Z"`
4. **Tag**: `git tag -a vX.Y.Z -m "Release vX.Y.Z: description"`
5. **Push**: `git push origin main && git push origin vX.Y.Z`

### Common Mistakes

- ❌ Tagging before final commit
- ❌ Skipping `cargo fmt --check`
- ❌ Not running full test suite
- ✅ Always follow the checklist in order

## Next Steps

- [Architecture Overview](./architecture.md) - Understand the codebase structure
- [Performance Guide](./performance.md) - Performance optimization tips
