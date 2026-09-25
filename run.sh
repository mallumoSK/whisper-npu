#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOCKET="/run/whisper-npu/daemon.sock"

# 1. Ensure socket directory exists
if [ ! -d "/run/whisper-npu" ]; then
  sudo mkdir -p /run/whisper-npu
  sudo chown -R "$USER:$USER" /run/whisper-npu
fi

# 2. Check if daemon is already running
DAEMON_RUNNING=0
if pgrep -x "whisper-daemon" > /dev/null 2>&1; then
  DAEMON_RUNNING=1
fi

if [ "$DAEMON_RUNNING" -eq 0 ]; then
  echo "whisper-daemon is not running. Starting daemon in background..."
  nohup "$SCRIPT_DIR/scripts/run_daemon.sh" > /tmp/whisper-daemon.log 2>&1 &
  disown $! || true
  echo "Waiting for daemon to initialize socket..."
  for i in {1..30}; do
    if [ -S "$SOCKET" ]; then
      echo "Daemon socket ready!"
      break
    fi
    sleep 0.2
  done
else
  echo "whisper-daemon is already running."
fi

# 3. Launch client
echo "Launching whisper-client..."
exec "$SCRIPT_DIR/scripts/run_client.sh" "$@"
