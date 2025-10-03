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

1. Update version in `Cargo.toml`
2. Create and push a tag: `git tag v0.1.1 && git push origin v0.1.1`
3. The CI will automatically build and publish the release

## Questions?

Feel free to open an issue for questions or discussions about contributing.

