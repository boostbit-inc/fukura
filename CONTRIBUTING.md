# Contributing to Fukura

Thank you for your interest in contributing to Fukura! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- Docker (optional, for containerized development)

### Setting Up the Development Environment

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/your-username/fukura.git
   cd fukura
   ```

2. Install dependencies:
   ```bash
   cargo build
   ```

3. Run tests to ensure everything works:
   ```bash
   cargo test
   ```

## Development Workflow

### Code Style

- Follow Rust's official style guidelines
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues
- Write meaningful commit messages following conventional commits

### Testing

- Write tests for new functionality
- Ensure all tests pass: `cargo test`
- Run integration tests: `cargo test --test integration`
- Run performance tests: `cargo test --test performance`
- Run security tests: `cargo test --test security`

### Benchmarks

- Add benchmarks for performance-critical code
- Run benchmarks: `cargo bench`
- Compare performance before/after changes

## Pull Request Process

1. Create a feature branch from `main`
2. Make your changes with appropriate tests
3. Ensure all CI checks pass
4. Submit a pull request with a clear description

### PR Requirements

- [ ] All tests pass
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Security audit passes (`cargo audit`)
- [ ] License compliance passes (`cargo deny check`)
- [ ] Documentation is updated if needed
- [ ] Benchmarks show no significant performance regression

## Security

- Report security vulnerabilities privately to security@fukura.dev
- Follow responsible disclosure practices
- Review security test results before submitting PRs

## Docker Development

For containerized development:

```bash
# Build and run tests in Docker
docker-compose run fukura cargo test

# Run the CLI in a container
docker-compose run fukura fukura --help
```

## Release Process

Releases are automated through GitHub Actions. To trigger a release:

### For Maintainers

1. **Update Version**
   ```bash
   # Update version in Cargo.toml
   # Example: version = "0.2.0"
   ```

2. **Create Release Tag**
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

3. **Monitor Release Process**
   - Check GitHub Actions tab for release workflow progress
   - Verify all build jobs complete successfully
   - Confirm artifacts are uploaded to GitHub Releases

### What Happens During Release

The release process automatically:

1. **Builds Multi-Platform Packages**
   - Linux: `.deb` (Debian/Ubuntu), `.rpm` (RedHat/CentOS), `.tar.gz`
   - Future: macOS `.pkg`, Windows `.msi` (when signing certificates are available)

2. **Signs Packages** (if GPG keys are configured)
   - Ensures package authenticity and integrity
   - Required for APT repository distribution

3. **Updates APT Repository**
   - Makes packages available via `apt install fukura`
   - Users get automatic updates through package manager

4. **Publishes Docker Images**
   - Updates `ghcr.io/boostbit-inc/fukura:latest`
   - Multi-platform support (AMD64/ARM64)

5. **Deploys Website**
   - Updates `fukura.dev` with latest version
   - Installation instructions reflect new release

### Release Checklist

Before creating a release, ensure:

- [ ] All tests pass (`cargo test`)
- [ ] Performance tests show no regression
- [ ] Security audit passes (`cargo audit`)
- [ ] Documentation is up to date
- [ ] Version number follows semantic versioning
- [ ] CHANGELOG.md is updated (if maintained)

## Questions?

Feel free to open an issue for questions or discussions about contributing.

