#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "Building julesctl for Android / Termux (aarch64)..."

# Ensure the target is installed
rustup target add aarch64-linux-android

# Build the project in release mode for the specified target
# Note: Cross-compiling for Android usually requires the Android NDK to be installed and configured.
cargo build --release --target aarch64-linux-android

echo "Build completed."
echo "The executable is located at target/aarch64-linux-android/release/julesctl"
