#!/bin/bash
# ============================================================================
# Dora Manager Installer
# ============================================================================
# One-line install:
#   curl -fsSL https://raw.githubusercontent.com/l1veIn/dora-manager/master/scripts/install.sh | bash
#
# With options:
#   curl -fsSL ... | bash -s -- --skip-setup
# ============================================================================

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# Configuration
REPO="l1veIn/dora-manager"
DM_HOME="${DM_HOME:-$HOME/.dm}"
BIN_DIR="${DM_BIN_DIR:-$HOME/.local/bin}"
VERSION="${DM_VERSION:-latest}"

# Options
RUN_SETUP=true
SKIP_DEPS=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-setup)  RUN_SETUP=false; shift ;;
        --skip-deps)   SKIP_DEPS=true; shift ;;
        --version)     VERSION="$2"; shift 2 ;;
        --bin-dir)     BIN_DIR="$2"; shift 2 ;;
        -h|--help)
            echo "Dora Manager Installer"
            echo ""
            echo "Usage: install.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --skip-setup   Skip 'dm setup' (dora-rs installation)"
            echo "  --skip-deps    Skip dependency checks (Rust, Python, uv)"
            echo "  --version VER  Install specific version (default: latest)"
            echo "  --bin-dir PATH Binary install directory (default: ~/.local/bin)"
            echo "  -h, --help     Show this help"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# ============================================================================
# Helpers
# ============================================================================

info()    { echo -e "${CYAN}→${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
warn()    { echo -e "${YELLOW}⚠${NC} $1"; }
error()   { echo -e "${RED}✗${NC} $1"; }

print_banner() {
    echo ""
    echo -e "${CYAN}${BOLD}"
    echo "  ┌─────────────────────────────────────────┐"
    echo "  │        dm — Dora Manager Installer      │"
    echo "  │  Dataflow orchestration for dora-rs     │"
    echo "  └─────────────────────────────────────────┘"
    echo -e "${NC}"
}

# ============================================================================
# System detection
# ============================================================================

detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="macos" ;;
        *)
            error "Unsupported OS: $(uname -s)"
            echo "  Supported: Linux, macOS"
            exit 1
            ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)  arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac

    if [ "$os" = "macos" ]; then
        PLATFORM="aarch64-apple-darwin"
        if [ "$arch" = "x86_64" ]; then
            PLATFORM="x86_64-apple-darwin"
        fi
    else
        PLATFORM="${arch}-unknown-linux-gnu"
    fi

    success "Detected: $os ($arch) → $PLATFORM"
}

# ============================================================================
# Download binary
# ============================================================================

download_dm() {
    local url filename tag

    if [ "$VERSION" = "latest" ]; then
        info "Fetching latest release..."
        tag=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"//' | sed 's/".*//')
        if [ -z "$tag" ]; then
            error "Could not determine latest release"
            exit 1
        fi
        VERSION="$tag"
    else
        tag="$VERSION"
    fi

    info "Downloading dora-manager $tag ($PLATFORM)..."

    filename="dora-manager-${tag}-${PLATFORM}.tar.gz"
    url="https://github.com/$REPO/releases/download/$tag/$filename"

    local tmpdir
    tmpdir=$(mktemp -d)
    trap "rm -rf $tmpdir" EXIT

    if ! curl -fsSL -o "$tmpdir/$filename" "$url"; then
        # Try without tag prefix (some releases use v0.1.0, others 0.1.0)
        filename="dora-manager-${tag#v}-${PLATFORM}.tar.gz"
        url="https://github.com/$REPO/releases/download/$tag/$filename"
        if ! curl -fsSL -o "$tmpdir/$filename" "$url"; then
            error "Download failed: $url"
            error "Check https://github.com/$REPO/releases for available assets"
            exit 1
        fi
    fi

    info "Extracting..."
    tar xzf "$tmpdir/$filename" -C "$tmpdir"

    mkdir -p "$BIN_DIR"
    cp "$tmpdir/dm" "$BIN_DIR/dm"
    cp "$tmpdir/dm-server" "$BIN_DIR/dm-server"
    chmod +x "$BIN_DIR/dm" "$BIN_DIR/dm-server"

    success "Installed dm $tag to $BIN_DIR/"
}

# ============================================================================
# Dependency checks
# ============================================================================

check_rust() {
    if [ "$SKIP_DEPS" = true ]; then return; fi

    info "Checking Rust..."
    if command -v cargo &>/dev/null; then
        success "Rust $(cargo --version | head -1)"
    else
        warn "Rust not found. Some node installations require cargo."
        echo "  Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "  Or run:  dm doctor  after installation to check all dependencies."
    fi
}

check_python() {
    if [ "$SKIP_DEPS" = true ]; then return; fi

    info "Checking Python + uv..."
    if command -v uv &>/dev/null; then
        success "uv $(uv --version 2>/dev/null | head -1)"
    elif command -v python3 &>/dev/null; then
        success "Python $(python3 --version)"
        warn "uv not found. Recommended for faster node installations:"
        echo "  Install: curl -LsSf https://astral.sh/uv/install.sh | sh"
    else
        warn "Python not found. Many nodes require Python 3.10+."
        echo "  Install uv (includes Python): curl -LsSf https://astral.sh/uv/install.sh | sh"
    fi
}

# ============================================================================
# PATH setup
# ============================================================================

setup_path() {
    # Check if BIN_DIR is in PATH
    case ":$PATH:" in
        *":$BIN_DIR:"*)
            # Already in PATH
            ;;
        *)
            warn "$BIN_DIR is not in your PATH."
            echo ""
            echo "  Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
            echo ""
            echo "    export PATH=\"\$PATH:$BIN_DIR\""
            echo ""
            echo "  Then run: source ~/.bashrc  (or restart your terminal)"
            ;;
    esac
}

# ============================================================================
# dm setup
# ============================================================================

run_dm_setup() {
    if [ "$RUN_SETUP" != true ]; then
        info "Skipping 'dm setup' (--skip-setup)"
        return
    fi

    info "Running dm setup (installing dora-rs runtime)..."
    "$BIN_DIR/dm" setup
    success "dora-rs runtime installed"
}

# ============================================================================
# Main
# ============================================================================

main() {
    print_banner

    detect_platform
    download_dm
    check_rust
    check_python
    setup_path
    run_dm_setup

    echo ""
    echo -e "${GREEN}${BOLD}  Installation complete!${NC}"
    echo ""
    echo "  Quick start:"
    echo "    $BIN_DIR/dm doctor              # Check environment"
    echo "    $BIN_DIR/dm-server              # Start web UI (port 3210)"
    echo "    $BIN_DIR/dm start demos/demo-hello-timer.yml"
    echo ""
    echo "  Documentation: https://github.com/$REPO"
    echo ""
}

main
