#!/usr/bin/env bash
set -euo pipefail
echo "Starting llama.service with sudo..."
sudo systemctl start llama.service
echo "Checking health..."
for i in {1..30}; do
  if curl -s -f http://127.0.0.1:8080/health > /dev/null 2>&1; then
    echo "llama.service is healthy and ready!"
    exit 0
  fi
  sleep 0.5
done
echo "Warning: Service started but health endpoint did not respond within 15s"
