#!/bin/bash
set -euo pipefail

# Development setup script for Fukura
# This script sets up the development environment

echo "🚀 Setting up Fukura development environment..."

# Check if Rust is installed
if ! command -v rustc >/dev/null 2>&1; then
    echo "❌ Rust is not installed. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✅ Rust is installed: $(rustc --version)"

# Install development tools
echo "📦 Installing development tools..."

# Install cargo-audit if not present
if ! command -v cargo-audit >/dev/null 2>&1; then
    echo "Installing cargo-audit..."
    cargo install cargo-audit
else
    echo "✅ cargo-audit is already installed"
fi

# Install cargo-deny if not present
if ! command -v cargo-deny >/dev/null 2>&1; then
    echo "Installing cargo-deny..."
    cargo install cargo-deny
else
    echo "✅ cargo-deny is already installed"
fi

# Install cargo-watch if not present
if ! command -v cargo-watch >/dev/null 2>&1; then
    echo "Installing cargo-watch..."
    cargo install cargo-watch
else
    echo "✅ cargo-watch is already installed"
fi

# Install cargo-expand if not present
if ! command -v cargo-expand >/dev/null 2>&1; then
    echo "Installing cargo-expand..."
    cargo install cargo-expand
else
    echo "✅ cargo-expand is already installed"
fi

# Run initial checks
echo "🔍 Running initial checks..."

# Format check
echo "Checking code formatting..."
cargo fmt --all -- --check

# Clippy check
echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
echo "Running tests..."
cargo test

# Security audit
echo "Running security audit..."
cargo audit

# Cargo-deny check
echo "Running cargo-deny check..."
cargo deny check

echo "🎉 Development environment setup complete!"
echo ""
echo "Available commands:"
echo "  make build      - Build the project"
echo "  make test       - Run tests"
echo "  make fmt        - Format code"
echo "  make clippy     - Run clippy lints"
echo "  make audit      - Run security audit"
echo "  make deny       - Run cargo-deny checks"
echo "  make clean      - Clean build artifacts"
echo "  make install    - Install fukura"
echo ""
echo "Development workflow:"
echo "  1. make fmt      # Format your code"
echo "  2. make clippy   # Check for linting issues"
echo "  3. make test     # Run tests"
echo "  4. make audit    # Check for security issues"
echo "  5. git commit    # Commit your changes"
