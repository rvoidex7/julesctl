#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "Building julesctl for Linux ARM64 (Raspberry Pi/ARM PCs)..."

# Ensure the target is installed
rustup target add aarch64-unknown-linux-gnu

# Build the project in release mode for the specified target
cargo build --release --target aarch64-unknown-linux-gnu

echo "Build completed."
echo "The executable is located at target/aarch64-unknown-linux-gnu/release/julesctl"
