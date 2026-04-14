#!/bin/bash
# Development workflow script for dora-manager
# Starts dm-server (Rust backend) and web dev server (SvelteKit) together.
# Ctrl+C stops both.

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

info()  { echo -e "${CYAN}==>${RESET} ${BOLD}$1${RESET}"; }
ok()    { echo -e "${GREEN}==>${RESET} $1"; }
warn()  { echo -e "${YELLOW}==>${RESET} $1"; }
die()   { echo -e "${RED}==>${RESET} $1" >&2; exit 1; }

# ── Preflight checks ────────────────────────────────────────────────
info "Checking prerequisites..."

if ! command -v cargo &>/dev/null; then
    die "Rust (cargo) is not installed. Install it from https://rustup.rs"
fi
ok "Rust $(cargo --version | head -1)"

if ! command -v node &>/dev/null; then
    die "Node.js is not installed. Install it from https://nodejs.org"
fi
ok "Node.js $(node --version)"

if ! command -v npm &>/dev/null; then
    die "npm is not found. Ensure Node.js is properly installed."
fi

# ── Build web frontend ──────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEB_DIR="$SCRIPT_DIR/web"

if [ -d "$WEB_DIR" ]; then
    cd "$WEB_DIR"

    if [ ! -d "node_modules" ]; then
        info "Installing web dependencies (first time)..."
        npm install
    fi

    info "Building web frontend..."
    npm run build
    ok "Web frontend built."

    cd "$SCRIPT_DIR"
else
    warn "web/ directory not found, skipping frontend build."
fi

# ── Cleanup ─────────────────────────────────────────────────────────
SERVER_PID=""
WEB_PID=""

cleanup() {
    echo ""
    info "Shutting down..."
    [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null
    [ -n "$WEB_PID" ]    && kill "$WEB_PID" 2>/dev/null
    wait "$SERVER_PID" "$WEB_PID" 2>/dev/null
    ok "Done."
}
trap cleanup EXIT INT TERM

# ── Start servers ───────────────────────────────────────────────────
info "Starting dm-server (port 3210)..."
cargo run -p dm-server &
SERVER_PID=$!

# Wait for server to begin listening
sleep 2

if [ -d "$WEB_DIR" ]; then
    info "Starting web dev server..."
    cd "$WEB_DIR" && npm run dev &
    WEB_PID=$!
fi

echo ""
ok "Both servers running. Press Ctrl+C to stop."
wait
