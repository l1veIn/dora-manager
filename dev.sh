#!/bin/bash
# Development workflow script for dora-manager
# Starts dm-server (Rust backend) and the web dev server (SvelteKit) together.
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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEB_DIR="$SCRIPT_DIR/web"
BACKEND_PORT=3210
FRONTEND_PORT=5173

port_listener_pids() {
    local port="$1"
    lsof -nP -t -iTCP:"$port" -sTCP:LISTEN 2>/dev/null | sort -u
}

port_listener_details() {
    local port="$1"
    lsof -nP -iTCP:"$port" -sTCP:LISTEN 2>/dev/null || true
}

process_command() {
    local pid="$1"
    ps -p "$pid" -o command= 2>/dev/null | sed 's/^ *//'
}

is_dm_server_process() {
    local pid="$1"
    local command
    command="$(process_command "$pid")"
    [[ "$command" == *"dm-server"* ]]
}

wait_for_port() {
    local port="$1"
    local pid="$2"
    local label="$3"
    local deadline=$((SECONDS + 30))

    while [ "$SECONDS" -lt "$deadline" ]; do
        if port_listener_pids "$port" >/dev/null; then
            ok "$label is listening on port $port"
            return 0
        fi

        if ! kill -0 "$pid" 2>/dev/null; then
            return 1
        fi

        sleep 0.25
    done

    return 1
}

wait_for_frontend_url() {
    local log_file="$1"
    local pid="$2"
    local deadline=$((SECONDS + 30))

    while [ "$SECONDS" -lt "$deadline" ]; do
        if [ -f "$log_file" ]; then
            local url
            url="$(grep -Eo 'http://(127\.0\.0\.1|localhost):[0-9]+' "$log_file" | tail -1)"
            if [ -n "$url" ]; then
                echo "$url"
                return 0
            fi
        fi

        if ! kill -0 "$pid" 2>/dev/null; then
            return 1
        fi

        sleep 0.25
    done

    return 1
}

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
if [ -d "$WEB_DIR" ]; then
    cd "$WEB_DIR"

    if [ ! -d "node_modules" ]; then
        info "Installing web dependencies (first time)..."
        npm install
    fi

    info "Preparing web frontend..."
    # npm run build
    ok "Web frontend is ready."

    cd "$SCRIPT_DIR"
else
    warn "web/ directory not found, skipping frontend preparation."
fi

# ── Cleanup ─────────────────────────────────────────────────────────
SERVER_PID=""
WEB_PID=""
WEB_TEE_PID=""
WEB_FIFO=""
TMP_DIR=""
BACKEND_REUSED=false

cleanup() {
    echo ""
    info "Shutting down..."
    [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null
    [ -n "$WEB_PID" ] && kill "$WEB_PID" 2>/dev/null
    [ -n "$WEB_TEE_PID" ] && kill "$WEB_TEE_PID" 2>/dev/null
    if [ -n "$SERVER_PID" ] || [ -n "$WEB_PID" ]; then
        wait ${SERVER_PID:+"$SERVER_PID"} ${WEB_PID:+"$WEB_PID"} 2>/dev/null || true
    fi
    [ -n "$WEB_FIFO" ] && rm -f "$WEB_FIFO"
    [ -n "$TMP_DIR" ] && rm -rf "$TMP_DIR"
    ok "Done."
}
trap cleanup EXIT INT TERM

# ── Start servers ───────────────────────────────────────────────────
backend_pids="$(port_listener_pids "$BACKEND_PORT")"
backend_already_running=false

if [ -n "$backend_pids" ]; then
    while read -r pid; do
        [ -z "$pid" ] && continue
        if is_dm_server_process "$pid"; then
            backend_already_running=true
            break
        fi
    done <<EOF
$backend_pids
EOF

    if $backend_already_running; then
        BACKEND_REUSED=true
        warn "Port $BACKEND_PORT is already served by an existing dm-server. Reusing it and starting only the frontend dev server."
        warn "Frontend requests will still proxy to http://127.0.0.1:$BACKEND_PORT."
    else
        die "Port $BACKEND_PORT is already in use.

$(port_listener_details "$BACKEND_PORT")

Stop the process above or free the port, then rerun ./dev.sh."
    fi
else
    info "Starting dm-server (port $BACKEND_PORT)..."
    cargo run -p dm-server &
    SERVER_PID=$!

    if ! wait_for_port "$BACKEND_PORT" "$SERVER_PID" "dm-server"; then
        wait "$SERVER_PID" 2>/dev/null || true
        die "dm-server did not come up on port $BACKEND_PORT.

Check the cargo output above for the build or bind error, then rerun ./dev.sh."
    fi
fi

if [ -d "$WEB_DIR" ]; then
    info "Starting web dev server (default port $FRONTEND_PORT; Vite may choose the next free port if it is busy)..."

    TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/dora-manager-dev.XXXXXX")"
    WEB_FIFO="$TMP_DIR/web-dev.fifo"
    WEB_LOG="$TMP_DIR/web-dev.log"
    mkfifo "$WEB_FIFO"
    tee "$WEB_LOG" <"$WEB_FIFO" &
    WEB_TEE_PID=$!

    (
        cd "$WEB_DIR" && npm run dev -- --host 127.0.0.1 --port "$FRONTEND_PORT"
    ) >"$WEB_FIFO" 2>&1 &
    WEB_PID=$!

    frontend_url="$(wait_for_frontend_url "$WEB_LOG" "$WEB_PID" || true)"
    if [ -n "$frontend_url" ]; then
        if [ "$frontend_url" != "http://127.0.0.1:$FRONTEND_PORT" ] && [ "$frontend_url" != "http://localhost:$FRONTEND_PORT" ]; then
            warn "Frontend dev server moved to $frontend_url because port $FRONTEND_PORT was already in use."
        else
            ok "Frontend dev server listening at $frontend_url."
        fi
    else
        warn "Frontend dev server is starting; check the Vite output above for the exact URL."
    fi
fi

echo ""
if $BACKEND_REUSED && [ -n "$WEB_PID" ]; then
    ok "Frontend dev server is running and an existing backend is still serving port $BACKEND_PORT. Press Ctrl+C to stop."
elif [ -n "$SERVER_PID" ] && [ -n "$WEB_PID" ]; then
    ok "Backend and frontend dev servers are running. Press Ctrl+C to stop."
elif [ -n "$WEB_PID" ]; then
    ok "Frontend dev server is running. Press Ctrl+C to stop."
else
    ok "Backend dev server is already running. Press Ctrl+C to stop when you are done."
fi
wait
