#!/bin/bash
set -e

echo "Building Muxspace AppImage..."

# Install cargo-appimage if not present
if ! command -v cargo-appimage &> /dev/null; then
    cargo install cargo-appimage
fi

# Build release
cargo build --release

# Create AppDir
mkdir -p AppDir/usr/bin
mkdir -p AppDir/usr/share/applications
mkdir -p AppDir/usr/share/icons/hicolor/256x256/apps

# Copy binary
cp target/release/muxspace-dioxus AppDir/usr/bin/muxspace

# Create desktop entry
cat > AppDir/usr/share/applications/muxspace.desktop << 'EOF'
[Desktop Entry]
Name=Muxspace
Exec=muxspace
Icon=muxspace
Type=Application
Categories=Development;System;TerminalEmulator;
Comment=Terminal Workspace Manager
EOF

# Create icon placeholder (would need actual icon)
touch AppDir/usr/share/icons/hicolor/256x256/apps/muxspace.png

# Build AppImage
cargo appimage

echo "AppImage built successfully!"
