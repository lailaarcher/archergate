#!/bin/bash
# Build archergate-license pre-compiled binaries for all platforms
# Run this on your respective build machines or CI/CD pipeline

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE_DIR="$REPO_ROOT/release-artifacts"
VERSION="0.1.0"

mkdir -p "$RELEASE_DIR"

echo "Building archergate-license v$VERSION pre-compiled binaries..."

# ─────────────────────────────────────────────────────────────────
# Platform: Windows (MSVC)
# Run this on: Windows with MSVC installed
# ─────────────────────────────────────────────────────────────────

build_windows() {
    echo "Building Windows (MSVC) binaries..."
    cd "$REPO_ROOT"

    cargo build --release -p archergate-license

    # Create Windows bundle
    mkdir -p "$RELEASE_DIR/windows-x64"

    # Determine target directory (check .cargo/config.toml or use default)
    TARGET_DIR="${CARGO_TARGET_DIR:-target}"
    if [ ! -d "$TARGET_DIR/release" ]; then
        # Try checking .cargo/config.toml for custom target-dir
        if [ -f ".cargo/config.toml" ]; then
            TARGET_DIR=$(grep '^target-dir' .cargo/config.toml | cut -d'"' -f2 | sed 's|\\|/|g')
        fi
    fi

    # Copy binaries from the actual target directory
    find "$TARGET_DIR/release" -maxdepth 1 \( -name "archergate_license.dll" -o -name "archergate_license.lib" -o -name "*.a" \) -exec cp {} "$RELEASE_DIR/windows-x64/" \; 2>/dev/null || true

    cp crates/archergate-license/include/archergate_license.h "$RELEASE_DIR/windows-x64/"
    cp crates/archergate-license/README.md "$RELEASE_DIR/windows-x64/"

    # Create ZIP archive (cross-platform)
    cd "$RELEASE_DIR"
    if command -v zip &> /dev/null; then
        zip -r "archergate-license-v${VERSION}-windows-x64.zip" windows-x64/
    elif command -v tar &> /dev/null; then
        tar -czf "archergate-license-v${VERSION}-windows-x64.tar.gz" windows-x64/
    fi
    echo "✓ Windows binary: archergate-license-v${VERSION}-windows-x64.zip"
}

# ─────────────────────────────────────────────────────────────────
# Platform: macOS (Intel + Apple Silicon)
# Run this on: macOS
# ─────────────────────────────────────────────────────────────────

build_macos() {
    echo "Building macOS binaries..."
    cd "$REPO_ROOT"

    mkdir -p "$RELEASE_DIR/macos-universal"

    # Intel
    echo "  Building x86_64-apple-darwin..."
    cargo build --release --target x86_64-apple-darwin -p archergate-license
    cp target/x86_64-apple-darwin/release/*.a "$RELEASE_DIR/macos-universal/libarchergate_license-x86_64.a" 2>/dev/null || true

    # Apple Silicon
    echo "  Building aarch64-apple-darwin..."
    cargo build --release --target aarch64-apple-darwin -p archergate-license
    cp target/aarch64-apple-darwin/release/*.a "$RELEASE_DIR/macos-universal/libarchergate_license-arm64.a" 2>/dev/null || true

    # Create universal binary if lipo is available
    if command -v lipo &> /dev/null; then
        lipo -create \
            "$RELEASE_DIR/macos-universal/libarchergate_license-x86_64.a" \
            "$RELEASE_DIR/macos-universal/libarchergate_license-arm64.a" \
            -output "$RELEASE_DIR/macos-universal/libarchergate_license.a"
        rm "$RELEASE_DIR/macos-universal/libarchergate_license-*.a"
        echo "  Created universal binary (x86_64 + arm64)"
    fi

    cp crates/archergate-license/include/archergate_license.h "$RELEASE_DIR/macos-universal/"
    cp crates/archergate-license/README.md "$RELEASE_DIR/macos-universal/"

    # Create ZIP archive (cross-platform)
    cd "$RELEASE_DIR"
    if command -v zip &> /dev/null; then
        zip -r "archergate-license-v${VERSION}-macos-universal.zip" macos-universal/
    elif command -v tar &> /dev/null; then
        tar -czf "archergate-license-v${VERSION}-macos-universal.tar.gz" macos-universal/
    fi
    echo "✓ macOS binary: archergate-license-v${VERSION}-macos-universal.zip"
}

# ─────────────────────────────────────────────────────────────────
# Platform: Linux (x86_64)
# Run this on: Linux
# ─────────────────────────────────────────────────────────────────

build_linux() {
    echo "Building Linux (x86_64) binaries..."
    cd "$REPO_ROOT"

    cargo build --release -p archergate-license

    mkdir -p "$RELEASE_DIR/linux-x64"

    # Copy static library and .so if built
    find target/release -maxdepth 1 -name "*.a" -exec cp {} "$RELEASE_DIR/linux-x64/libarchergate_license.a" \; 2>/dev/null || true
    find target/release -maxdepth 1 -name "*.so*" -exec cp {} "$RELEASE_DIR/linux-x64/" \; 2>/dev/null || true

    cp crates/archergate-license/include/archergate_license.h "$RELEASE_DIR/linux-x64/"
    cp crates/archergate-license/README.md "$RELEASE_DIR/linux-x64/"

    # Create ZIP archive (cross-platform)
    cd "$RELEASE_DIR"
    if command -v zip &> /dev/null; then
        zip -r "archergate-license-v${VERSION}-linux-x64.zip" linux-x64/
    elif command -v tar &> /dev/null; then
        tar -czf "archergate-license-v${VERSION}-linux-x64.tar.gz" linux-x64/
    fi
    echo "✓ Linux binary: archergate-license-v${VERSION}-linux-x64.zip"
}

# ─────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────

case "${1:-all}" in
    windows)
        build_windows
        ;;
    macos)
        build_macos
        ;;
    linux)
        build_linux
        ;;
    all)
        echo "Usage: $0 [windows|macos|linux|all]"
        echo ""
        echo "This script should be run on the respective platform:"
        echo "  - Run './scripts/build-release-binaries.sh windows' on Windows"
        echo "  - Run './scripts/build-release-binaries.sh macos' on macOS"
        echo "  - Run './scripts/build-release-binaries.sh linux' on Linux"
        echo ""
        echo "Or run './scripts/build-release-binaries.sh all' to build all (requires all platforms)"
        exit 0
        ;;
    *)
        echo "Unknown platform: $1"
        exit 1
        ;;
esac

# ─────────────────────────────────────────────────────────────────
# Create SHA256 checksums
# ─────────────────────────────────────────────────────────────────

echo ""
echo "Creating SHA256 checksums..."
cd "$RELEASE_DIR"

# Use sha256sum if available (Linux/macOS), fallback to openssl (macOS)
if command -v sha256sum &> /dev/null; then
    sha256sum *.zip *.tar.gz 2>/dev/null > SHA256SUMS
elif command -v shasum &> /dev/null; then
    shasum -a 256 *.zip *.tar.gz 2>/dev/null > SHA256SUMS
fi

cat SHA256SUMS
echo ""
echo "✓ All checksums saved to SHA256SUMS"
echo ""
echo "Artifacts ready for GitHub release:"
ls -lh "$RELEASE_DIR"/*.{zip,tar.gz} 2>/dev/null || ls -lh "$RELEASE_DIR"/*.zip 2>/dev/null
