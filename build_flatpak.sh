#!/bin/bash
set -e

echo "Building Muxspace Flatpak..."

# Generate cargo-sources.json
cd ..
if ! command -v python3 &> /dev/null; then
    echo "Python3 is required for Flatpak build"
    exit 1
fi

# Download flatpak-cargo-generator.py if not present
if [ ! -f flatpak-cargo-generator.py ]; then
    wget https://github.com/flatpak/flatpak-builder-tools/raw/master/cargo/flatpak-cargo-generator.py
    chmod +x flatpak-cargo-generator.py
fi

# Generate cargo sources
python3 flatpak-cargo-generator.py Cargo.lock -o dioxus/flatpak/cargo-sources.json

# Build Flatpak
cd dioxus/flatpak
flatpak-builder --force-clean build-dir com.muxspace.Muxspace.yml

# Create repo
flatpak-builder --repo=repo --force-clean build-dir com.muxspace.Muxspace.yml

# Build bundle
flatpak build-bundle repo muxspace.flatpak com.muxspace.Muxspace

echo "Flatpak bundle created: muxspace.flatpak"
echo "Install with: flatpak install --user muxspace.flatpak"
