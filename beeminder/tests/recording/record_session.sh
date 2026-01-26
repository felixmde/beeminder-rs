#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== Beeminder API Recording Session ==="
echo ""
echo "This script starts mitmproxy to record API responses."
echo ""

# Check for danger flag
if [[ "$1" != "--include-danger" ]]; then
    echo "Skipping danger endpoints. Use --include-danger to record."
    export SKIP_DANGER=1
else
    echo "WARNING: Danger endpoints ENABLED. Verify $0 pledge goals first!"
    read -p "Continue? [y/N] " confirm
    if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
        echo "Aborted."
        exit 1
    fi
fi

echo ""
echo "Starting mitmproxy on port 8080..."
echo "Set HTTPS_PROXY=http://localhost:8080 to capture traffic."
echo ""
echo "Example:"
echo "  HTTPS_PROXY=http://localhost:8080 cargo run -p beeminder --example demo"
echo ""

mitmdump -s "$SCRIPT_DIR/mitmproxy_script.py"
