#!/bin/sh
set -e

REPO="twilight-project/nyks-wallet"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"
INSTALL_DIR="."
BINARY_NAME="relayer-cli"

# --- Detect platform --------------------------------------------------------

OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
    Linux)  ARTIFACT="nyks-wallet-linux-amd64" ;;
    Darwin) ARTIFACT="nyks-wallet-macos-arm64" ;;
    *)      echo "Error: unsupported OS: ${OS}" >&2; exit 1 ;;
esac

case "${ARCH}" in
    x86_64|amd64)
        if [ "${OS}" = "Linux" ]; then
            ARTIFACT="nyks-wallet-linux-amd64"
        else
            echo "Error: macOS x86_64 is not supported (Apple Silicon only)" >&2; exit 1
        fi
        ;;
    arm64|aarch64)
        if [ "${OS}" = "Darwin" ]; then
            ARTIFACT="nyks-wallet-macos-arm64"
        else
            ARTIFACT="nyks-wallet-linux-arm64"
        fi
        ;;
    *)
        echo "Error: unsupported architecture: ${ARCH}" >&2; exit 1
        ;;
esac

echo "Detected platform: ${OS} ${ARCH} -> ${ARTIFACT}"

# --- Fetch latest release URL -----------------------------------------------

echo "Fetching latest release from ${REPO}..."

RELEASE_JSON="$(curl -sf "${API_URL}")" || {
    echo "Error: failed to fetch release info from GitHub API" >&2
    exit 1
}

# Extract download URL and tag via grep — works regardless of JSON body encoding
DOWNLOAD_URL="$(echo "${RELEASE_JSON}" | grep "browser_download_url" | grep "${ARTIFACT}" | head -1 | cut -d '"' -f 4)"
TAG="$(echo "${RELEASE_JSON}" | grep '"tag_name"' | head -1 | cut -d '"' -f 4)"

if [ -z "${DOWNLOAD_URL}" ] || [ "${DOWNLOAD_URL}" = "null" ]; then
    echo "Error: could not find asset '${ARTIFACT}' in latest release" >&2
    exit 1
fi

echo "Downloading ${TAG} (${ARTIFACT})..."

# --- Download and install ----------------------------------------------------

curl -sfL "${DOWNLOAD_URL}" -o "${INSTALL_DIR}/${BINARY_NAME}" || {
    echo "Error: download failed" >&2
    exit 1
}

chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

echo ""
echo "Installed ${BINARY_NAME} ${TAG} to ${INSTALL_DIR}/${BINARY_NAME}"
echo "Run ./${BINARY_NAME} --help to get started."
