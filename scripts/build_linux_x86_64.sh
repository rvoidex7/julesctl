#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "Building julesctl for Linux x86_64 (Standard 64-bit Linux)..."

# Ensure the target is installed
rustup target add x86_64-unknown-linux-gnu

# Build the project in release mode for the specified target
cargo build --release --target x86_64-unknown-linux-gnu

echo "Build completed."
echo "The executable is located at target/x86_64-unknown-linux-gnu/release/julesctl"
