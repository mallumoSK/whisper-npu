#!/usr/bin/env bash
sudo systemctl status llama.service || true
echo -n "Health endpoint: "
curl -s http://127.0.0.1:8080/health || echo "Offline"
echo
