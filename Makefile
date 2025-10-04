.PHONY: help build test fmt clippy audit deny clean install dev-setup

# Default target
help:
	@echo "Available targets:"
	@echo "  build      - Build the project"
	@echo "  test       - Run tests"
	@echo "  fmt        - Format code"
	@echo "  clippy     - Run clippy lints"
	@echo "  audit      - Run security audit"
	@echo "  deny       - Run cargo-deny checks"
	@echo "  clean      - Clean build artifacts"
	@echo "  install    - Install fukura"
	@echo "  dev-setup  - Setup development environment"

# Build the project
build:
	cargo build --release

# Run tests
test:
	cargo test

# Format code
fmt:
	cargo fmt --all

# Run clippy
clippy:
	cargo clippy --all-targets --all-features -- -D warnings

# Security audit
audit:
	cargo audit

# Cargo-deny checks
deny:
	cargo deny check

# Clean build artifacts
clean:
	cargo clean

# Install fukura
install:
	cargo install --path .

# Development setup
dev-setup:
	@echo "Setting up development environment..."
	@if ! command -v cargo-audit >/dev/null 2>&1; then \
		echo "Installing cargo-audit..."; \
		cargo install cargo-audit; \
	fi
	@if ! command -v cargo-deny >/dev/null 2>&1; then \
		echo "Installing cargo-deny..."; \
		cargo install cargo-deny; \
	fi
	@echo "Development environment setup complete!"

# CI check (run all checks)
ci: fmt clippy test audit deny

# Pre-commit hook
pre-commit: fmt clippy test

# Release preparation
release-prep: clean fmt clippy test audit deny build
