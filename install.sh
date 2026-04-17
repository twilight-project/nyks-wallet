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

# Pick the newest release whose `.sha256` asset for this platform is already
# uploaded, then derive the binary URL from it. The release workflow uploads
# the binary first and the checksum second, so a present `.sha256` means the
# binary is fully uploaded too. This makes us skip releases whose build is
# still in progress (the release exists but assets aren't uploaded yet) and
# fall back to the previous fully-published release.
CHECKSUM_URL="$(echo "${RELEASES_JSON}" \
    | grep -oE "https://github\.com/[^\"]+_relayer_cli${PLATFORM_SUFFIX}\.sha256" \
    | head -1)"

DOWNLOAD_URL="${CHECKSUM_URL%.sha256}"

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
