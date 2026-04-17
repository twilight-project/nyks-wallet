#!/bin/sh
set -e

REPO="twilight-project/nyks-wallet"
API_URL="https://api.github.com/repos/${REPO}/releases?per_page=100"
INSTALL_DIR="."
BINARY_NAME="relayer-cli"

# --- Detect platform --------------------------------------------------------

OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
    Linux)
        case "${ARCH}" in
            x86_64|amd64)    PLATFORM_SUFFIX="_linux_amd64" ;;
            arm64|aarch64)   PLATFORM_SUFFIX="_linux_arm64" ;;
            *) echo "Error: unsupported Linux architecture: ${ARCH}" >&2; exit 1 ;;
        esac
        ;;
    Darwin)
        case "${ARCH}" in
            arm64|aarch64)   PLATFORM_SUFFIX="_macos_arm64" ;;
            *) echo "Error: macOS x86_64 is not supported (Apple Silicon only)" >&2; exit 1 ;;
        esac
        ;;
    *)
        echo "Error: unsupported OS: ${OS}" >&2; exit 1
        ;;
esac

echo "Detected platform: ${OS} ${ARCH} -> ${PLATFORM_SUFFIX}"

# --- Fetch releases list ----------------------------------------------------

echo "Fetching releases from ${REPO}..."

RELEASES_JSON="$(curl -sf "${API_URL}")" || {
    echo "Error: failed to fetch releases from GitHub API" >&2
    exit 1
}

# Pick the newest download URL whose filename matches the relayer-cli asset
# for this platform. Works regardless of whether GitHub returns pretty-printed
# or compact JSON: we extract the URL with a single regex rather than relying
# on line-by-line grep + field-cut. GitHub returns releases newest-first, so
# the first match is the latest relayer-cli release, which naturally ignores
# unrelated tags like v0.1.1 or v0.0.4-relayer-deployer.
#
# The regex stops at `_relayer_cli${PLATFORM_SUFFIX}` (without `.sha256`), so
# both binary and checksum URLs in the JSON produce the same binary URL here.
DOWNLOAD_URL="$(echo "${RELEASES_JSON}" \
    | grep -oE "https://github\.com/[^\"]+_relayer_cli${PLATFORM_SUFFIX}" \
    | head -1)"

if [ -z "${DOWNLOAD_URL}" ]; then
    echo "Error: could not find a relayer-cli asset for platform ${PLATFORM_SUFFIX}" >&2
    exit 1
fi

# Extract the tag from the download URL path:
# https://github.com/OWNER/REPO/releases/download/<TAG>/<ASSET>
TAG="$(echo "${DOWNLOAD_URL}" | awk -F'/releases/download/' '{print $2}' | awk -F'/' '{print $1}')"
ARTIFACT="$(echo "${DOWNLOAD_URL}" | awk -F'/' '{print $NF}')"

echo "Downloading ${TAG} (${ARTIFACT})..."

# --- Download and install ----------------------------------------------------

curl -sfL "${DOWNLOAD_URL}" -o "${INSTALL_DIR}/${BINARY_NAME}" || {
    echo "Error: download failed" >&2
    exit 1
}

# --- Verify checksum ---------------------------------------------------------

CHECKSUM_URL="${DOWNLOAD_URL}.sha256"

echo "Verifying checksum..."

EXPECTED="$(curl -sfL "${CHECKSUM_URL}" 2>/dev/null | cut -d ' ' -f 1)"

if [ -z "${EXPECTED}" ]; then
    echo "Warning: no checksum file found for ${ARTIFACT}, skipping verification" >&2
else
    # shasum works on both macOS and Linux
    ACTUAL="$(shasum -a 256 "${INSTALL_DIR}/${BINARY_NAME}" | cut -d ' ' -f 1)"

    if [ "${ACTUAL}" != "${EXPECTED}" ]; then
        echo "Error: checksum mismatch!" >&2
        echo "  Expected: ${EXPECTED}" >&2
        echo "  Actual:   ${ACTUAL}" >&2
        rm -f "${INSTALL_DIR}/${BINARY_NAME}"
        exit 1
    fi

    echo "Checksum verified."
fi

# --- Finalize ----------------------------------------------------------------

chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

echo ""
echo "Installed ${BINARY_NAME} ${TAG} to ${INSTALL_DIR}/${BINARY_NAME}"
echo "Run ./${BINARY_NAME} --help to get started."
