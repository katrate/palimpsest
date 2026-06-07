#!/usr/bin/env bash
set -euo pipefail

REPO="katrate/palimpsest"
PACKAGE="palimpsest"
BINARY="palin"

# ─── Detect platform ────────────────────────────────────────────────────
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
  linux)  TARGET_OS="unknown-linux-gnu" ;;
  darwin) TARGET_OS="apple-darwin" ;;
  *)      echo "✗ Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64 | amd64) TARGET_ARCH="x86_64" ;;
  aarch64 | arm64) TARGET_ARCH="aarch64" ;;
  *) echo "✗ Unsupported architecture: $ARCH"; exit 1 ;;
esac

TARGET="${TARGET_ARCH}-${TARGET_OS}"

# ─── Determine install directory ────────────────────────────────────────
if [ -w "/usr/local/bin" ]; then
  INSTALL_DIR="/usr/local/bin"
else
  INSTALL_DIR="${HOME}/.local/bin"
  mkdir -p "$INSTALL_DIR"
fi

# ─── Download & extract ─────────────────────────────────────────────────
# Use the permanent release-latest redirect to avoid GitHub API rate limits
ARCHIVE_NAME="${PACKAGE}-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ARCHIVE_NAME}"
EXTRACTED_DIR="${PACKAGE}-${TARGET}"

echo "✦ Downloading ${ARCHIVE_NAME}..."

if ! command -v curl &>/dev/null; then
  echo "✗ curl is required"; exit 1
fi

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

curl -sSfL "$DOWNLOAD_URL" -o "${TMPDIR}/${ARCHIVE_NAME}"

echo "✦ Extracting..."
tar -xzf "${TMPDIR}/${ARCHIVE_NAME}" -C "$TMPDIR"

echo "✦ Installing to ${INSTALL_DIR}/${BINARY}..."
if command -v install &>/dev/null; then
  install -m 755 "${TMPDIR}/${EXTRACTED_DIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
else
  cp "${TMPDIR}/${EXTRACTED_DIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
  chmod 755 "${INSTALL_DIR}/${BINARY}"
fi

# ─── Done ───────────────────────────────────────────────────────────────
echo "✓ palin installed successfully at ${INSTALL_DIR}/${BINARY}"

# Remind if ~/.local/bin is not in PATH
if [ "${INSTALL_DIR}" = "${HOME}/.local/bin" ]; then
  case ":${PATH}:" in
    *:"${HOME}/.local/bin":*) ;;
    *)
      echo ""
      echo "  ⚠  ${HOME}/.local/bin is not in your PATH."
      echo "     Add this to your shell config (~/.bashrc, ~/.zshrc, etc.):"
      echo ""
      echo "        export PATH=\"\${HOME}/.local/bin:\${PATH}\""
      ;;
  esac
fi

echo ""
echo "  Run \`palin --help\` to get started"
