#!/usr/bin/env bash
set -euo pipefail
echo "Stopping llama.service with sudo..."
sudo systemctl stop llama.service
echo "llama.service stopped."
