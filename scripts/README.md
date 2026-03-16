# julesctl Build Scripts

This directory contains standalone build scripts to compile `julesctl` for various operating systems and architectures.

Each script works independently, so you can compile only the target you need for testing or deployment.

## Prerequisites
Before running any of these scripts, ensure you have Rust installed. If you are cross-compiling (e.g., compiling an ARM64 executable from an x86_64 machine, or a Windows executable from Linux), you may need specific linker toolchains installed on your host system.

The scripts automatically run `rustup target add <target>` to fetch the Rust standard library for the respective architecture.

## Available Scripts

### Windows
These `.bat` scripts are meant to be run natively on a Windows machine.
- `build_win_x86_64.bat`: Builds for standard 64-bit Windows PCs (`x86_64-pc-windows-msvc`).
- `build_win_arm64.bat`: Builds for ARM64-based Windows PCs, like newer Snapdragon laptops (`aarch64-pc-windows-msvc`).

### Linux
These `.sh` scripts are meant to be run natively on a Linux machine.
- `build_linux_x86_64.sh`: Builds for standard 64-bit Linux (`x86_64-unknown-linux-gnu`).
- `build_linux_arm64.sh`: Builds for ARM64-based Linux devices, like Raspberry Pi (`aarch64-unknown-linux-gnu`).

### Android / Termux
- `build_android_termux.sh`: Builds for aarch64 Android/Termux environments (`aarch64-linux-android`).
  *Note: Cross-compiling for Android generally requires having the Android NDK (Native Development Kit) installed and configured on your host machine to provide the correct C linker (clang).*

### macOS
- `build_macos_x86_64.sh`: Builds for Intel-based Macs (`x86_64-apple-darwin`).
- `build_macos_arm64.sh`: Builds for Apple Silicon (M1/M2/M3) Macs (`aarch64-apple-darwin`).
  *Note: Compiling macOS binaries generally requires running these scripts natively on a macOS machine. Cross-compiling for macOS from Linux or Windows requires complex toolchains (like osxcross) due to Apple's SDK restrictions.*

## Finding the Compiled Binaries
Once a build succeeds, the executable will be placed in the `target/<target-triple>/release/` directory in the root of the project.
