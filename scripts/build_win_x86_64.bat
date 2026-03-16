@echo off
cd /d "%~dp0\.."

echo Building julesctl for Windows x86_64...

:: Ensure the target is installed
rustup target add x86_64-pc-windows-msvc

:: Build the project in release mode for the specified target
cargo build --release --target x86_64-pc-windows-msvc

echo Build completed. The executable is located at target\x86_64-pc-windows-msvc\release\julesctl.exe
pause
