#!/bin/bash
# package.sh - Script to build and package msaada for distribution

# Ensure we start in the project root directory
cd "$(dirname "$0")"

# Build the release version
echo "Building release version..."
cargo build --release

# Check if build was successful
if [ ! -f "target/release/msaada" ]; then
  echo "Error: Build failed or executable not found!"
  exit 1
fi

# Get version from Cargo.toml
VERSION=$(grep '^version' Cargo.toml | sed 's/.*= *"\(.*\)".*/\1/')
echo "Packaging msaada version $VERSION"

# Create distribution directory if it doesn't exist
mkdir -p dist

# Create compressed archive for current platform
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCHIVE_NAME="msaada-$VERSION-$PLATFORM.tar.gz"

echo "Creating archive: dist/$ARCHIVE_NAME"
tar -czvf "dist/$ARCHIVE_NAME" -C target/release msaada

echo "Package created successfully:"
ls -lh "dist/$ARCHIVE_NAME"
echo ""
echo "To extract and use: tar -xzvf $ARCHIVE_NAME"