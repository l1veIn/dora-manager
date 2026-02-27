#!/bin/bash
# Start dm-server (Rust backend) and web dev server (SvelteKit) together.
# Ctrl+C stops both.

set -e

cleanup() {
    echo ""
    echo "Shutting down..."
    kill $SERVER_PID $WEB_PID 2>/dev/null
    wait $SERVER_PID $WEB_PID 2>/dev/null
    echo "Done."
}
trap cleanup EXIT INT TERM

echo "🚀 Starting dm-server (port 3210)..."
cargo run -p dm-server &
SERVER_PID=$!

# Wait a moment for the server to start
sleep 2

echo "🌐 Starting web dev server..."
cd web && npm run dev &
WEB_PID=$!

echo ""
echo "✅ Both servers running. Press Ctrl+C to stop."
wait
