#!/usr/bin/env bash
set -euo pipefail

BINARY="palin"
DATA_DIR="${HOME}/palimpsest"

echo "==> Uninstalling palin..."

REMOVED=false

# Check common install locations
for DIR in "/usr/local/bin" "${HOME}/.local/bin"; do
  if [ -f "${DIR}/${BINARY}" ]; then
    rm -f "${DIR}/${BINARY}"
    echo "  Removed ${DIR}/${BINARY}"
    REMOVED=true
  fi
done

if [ "$REMOVED" = false ]; then
  echo "  palin binary not found in /usr/local/bin or ~/.local/bin."
  echo "  If you installed it elsewhere, remove it manually."
else
  echo "OK palin uninstalled."
fi

# Check for common palimpsest data directories
for DATA_DIR in "${HOME}/.local/share/palimpsest" "${HOME}/Library/Application Support/palimpsest"; do
  if [ -d "$DATA_DIR" ]; then
    echo ""
    echo "  NOTE: Your palimpsest data (snapshots, history) is still at:"
    echo "    ${DATA_DIR}"
    echo "  To remove it too, run:  rm -rf ${DATA_DIR}"
    break
  fi
done

echo ""
echo "  Run 'palin --help' to verify it's gone (should say 'command not found')"
