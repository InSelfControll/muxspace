# Muxspace Development Tasks

# Build the application in release mode
build:
  #!/usr/bin/env bash
  set -euo pipefail
  if [[ -z "${IN_NIX_SHELL:-}" ]] && [[ -r /etc/os-release ]] && grep -q '^ID=nixos$' /etc/os-release; then
    exec nix develop path:. --command cargo build --release --features desktop --quiet
  fi
  exec cargo build --release --features desktop --quiet

# Run clippy for linting
lint:
  cargo clippy --all-targets --features desktop -- -D warnings

# Run tests
test:
  cargo test

# Build and run the application
run: build
  ./target/release/muxspace

# Format code with rustfmt
fmt:
  cargo fmt

# Clean build artifacts
clean:
  cargo clean

# Install system dependencies (Ubuntu/Debian)
install-deps-ubuntu:
  sudo apt-get update
  sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libssl-dev

# Install system dependencies (Fedora/RHEL)
install-deps-fedora:
  sudo dnf install -y webkit2gtk4.0-devel gtk3-devel libsoup-devel javascriptcoregtk4.0-devel

# Create a development build
dev:
  cargo build --features desktop

# Run the application in development mode
dev-run: dev
  ./target/debug/muxspace

# Check formatting
check-fmt:
  cargo fmt --check

# Run all quality checks (lint, test, format check)
quality: lint test check-fmt

# Default task - build the application
default: build
