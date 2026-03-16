#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "Building julesctl for macOS ARM64 (Apple Silicon M1/M2/M3)..."

# Ensure the target is installed
rustup target add aarch64-apple-darwin

# Build the project in release mode for the specified target
cargo build --release --target aarch64-apple-darwin

echo "Build completed."
echo "The executable is located at target/aarch64-apple-darwin/release/julesctl"
