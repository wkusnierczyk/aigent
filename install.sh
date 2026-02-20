#!/bin/sh
set -eu

# aigent install script
# Downloads the latest aigent binary from GitHub Releases.
# Usage: curl -fsSL https://raw.githubusercontent.com/wkusnierczyk/aigent/main/install.sh | sh

REPO="wkusnierczyk/aigent"
INSTALL_DIR="${HOME}/.local/bin"

# Detect OS
OS="$(uname -s)"
case "$OS" in
  Linux)  OS_TARGET="unknown-linux-gnu" ;;
  Darwin) OS_TARGET="apple-darwin" ;;
  *)      echo "Error: unsupported OS: $OS" >&2; exit 1 ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64)         ARCH_TARGET="x86_64" ;;
  aarch64|arm64)  ARCH_TARGET="aarch64" ;;
  *)              echo "Error: unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

TARGET="${ARCH_TARGET}-${OS_TARGET}"

# Get latest release tag by following the redirect from /releases/latest
VERSION=$(curl -fsSI "https://github.com/${REPO}/releases/latest" \
  | grep -i '^location:' | sed 's|.*/tag/||;s/[[:space:]]*$//')

if [ -z "$VERSION" ]; then
  echo "Error: failed to determine latest version" >&2
  exit 1
fi

# Download and install
ASSET="aigent-${VERSION}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
CHECKSUM_URL="https://github.com/${REPO}/releases/download/${VERSION}/checksums.txt"

echo "Installing aigent ${VERSION} for ${TARGET}..."
mkdir -p "$INSTALL_DIR"

TMPDIR=$(mktemp -d 2>/dev/null || mktemp -d -t aigent)
trap 'rm -rf "$TMPDIR"' EXIT

# Download archive and checksums
if ! curl -fsSL -o "${TMPDIR}/${ASSET}" "$URL"; then
  echo "Error: download failed — check that a release exists for ${TARGET}" >&2
  exit 1
fi

if curl -fsSL -o "${TMPDIR}/checksums.txt" "$CHECKSUM_URL" 2>/dev/null; then
  # Verify checksum
  EXPECTED=$(awk -v asset="${ASSET}" '$2 == asset {print $1}' "${TMPDIR}/checksums.txt")
  if [ -n "$EXPECTED" ]; then
    if command -v sha256sum > /dev/null 2>&1; then
      ACTUAL=$(sha256sum "${TMPDIR}/${ASSET}" | awk '{print $1}')
    elif command -v shasum > /dev/null 2>&1; then
      ACTUAL=$(shasum -a 256 "${TMPDIR}/${ASSET}" | awk '{print $1}')
    else
      echo "Error: neither sha256sum nor shasum is available, cannot verify checksum." >&2
      echo "Please install sha256sum or shasum and re-run this installer." >&2
      exit 1
    fi
    if [ "$ACTUAL" != "$EXPECTED" ]; then
      echo "Error: checksum verification failed" >&2
      echo "  Expected: $EXPECTED" >&2
      echo "  Actual:   $ACTUAL" >&2
      exit 1
    fi
    echo "Checksum verified."
  else
    echo "Warning: no checksum found for ${ASSET}, skipping verification" >&2
  fi
else
  echo "Warning: checksums.txt not available, skipping verification" >&2
fi

# Extract
tar xzf "${TMPDIR}/${ASSET}" -C "$INSTALL_DIR"
chmod +x "${INSTALL_DIR}/aigent"

# Verify
if "${INSTALL_DIR}/aigent" --version > /dev/null 2>&1; then
  INSTALLED_VERSION=$("${INSTALL_DIR}/aigent" --version)
  echo "Installed ${INSTALLED_VERSION} to ${INSTALL_DIR}/aigent"
else
  echo "Error: installation failed — binary not functional" >&2
  exit 1
fi

# PATH hint
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *) echo "Add ${INSTALL_DIR} to your PATH if not already present." ;;
esac
