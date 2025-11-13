#!/bin/bash
set -e

# Load environment variables from .env if it exists
if [ -f .env ]; then
    echo "Loading environment variables from .env..."
    export $(cat .env | grep -v '^#' | xargs)
fi

# Set default config path if not specified
CONFIG_PATH=${CONFIG_PATH:-config/example-config.yaml}

echo "Starting LLM Proxy Router..."
echo "Configuration: $CONFIG_PATH"
echo ""

# Run the server
cargo run --release
