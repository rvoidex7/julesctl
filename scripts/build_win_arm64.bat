@echo off
cd /d "%~dp0\.."

echo Building julesctl for Windows ARM64 (Snapdragon/ARM PCs)...

:: Ensure the target is installed
rustup target add aarch64-pc-windows-msvc

:: Build the project in release mode for the specified target
cargo build --release --target aarch64-pc-windows-msvc

echo Build completed. The executable is located at target\aarch64-pc-windows-msvc\release\julesctl.exe
pause
