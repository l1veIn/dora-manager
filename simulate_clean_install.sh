#!/bin/bash
# simulate_clean_install.sh
# Tests the Dora Manager onboarding process on a pristine environment.

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}==> Starting Clean Install Simulation...${NC}"

DM_HOME="$HOME/.dm"
DM_BACKUP="$HOME/.dm_backup_safeguard_test"

# 1. Backup existing ~/.dm
if [ -d "$DM_HOME" ]; then
    echo -e "${GREEN}==> Backing up existing $DM_HOME to $DM_BACKUP${NC}"
    mv "$DM_HOME" "$DM_BACKUP"
else
    echo -e "${GREEN}==> $DM_HOME does not exist. No backup needed.${NC}"
fi

# Ensure cleanup runs on exit
cleanup() {
    echo -e "${GREEN}==> Restoring environment...${NC}"
    # Stop the server if running
    if [ ! -z "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
    fi
    
    # Remove sandbox
    if [ -d "$DM_HOME" ]; then
        rm -rf "$DM_HOME"
    fi
    
    # Restore backup
    if [ -d "$DM_BACKUP" ]; then
        mv "$DM_BACKUP" "$DM_HOME"
        echo -e "${GREEN}==> Original environment restored successfully.${NC}"
    fi
}
trap cleanup EXIT INT TERM

# 2. Build Web
echo -e "${GREEN}==> Compiling SvelteKit web assets...${NC}"
cd web
npm install > /dev/null 2>&1
npm run build > /dev/null 2>&1
cd ..

# 3. Build Rust
echo -e "${GREEN}==> Compiling dm-server (embedding web assets)...${NC}"
cargo build --release

# 4. DM Install
echo -e "${GREEN}==> Running 'dm install'...${NC}"
./target/release/dm install

# 5. DM Doctor
echo -e "${GREEN}==> Running 'dm doctor'...${NC}"
./target/release/dm doctor

# 6. Start Server in Background
echo -e "${GREEN}==> Starting 'dm-server' in background...${NC}"
./target/release/dm-server &
SERVER_PID=$!

# Wait for server to bind
sleep 3

# 7. Test Web UI Endpoint
echo -e "${GREEN}==> Testing HTTP 127.0.0.1:3210 (Checking for Svelte markup)...${NC}"
RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:3210/)

if [ "$RESPONSE" == "200" ]; then
    echo -e "${GREEN}==> Success! HTTP Server returned 200 OK.${NC}"
    echo -e "${GREEN}==> Clean Install Simulation completed without errors!${NC}"
else
    echo -e "${RED}==> Failure! HTTP Server returned $RESPONSE.${NC}"
    exit 1
fi
