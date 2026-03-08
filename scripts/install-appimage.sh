#!/usr/bin/env bash
set -euo pipefail

APP_NAME="49modman"
APPIMAGE_SOURCE="${1:-}"

if [[ -z "${APPIMAGE_SOURCE}" ]]; then
  echo "Usage: scripts/install-appimage.sh <path-to-appimage>"
  exit 1
fi

if [[ ! -f "${APPIMAGE_SOURCE}" ]]; then
  echo "Error: file not found: ${APPIMAGE_SOURCE}"
  exit 1
fi

if [[ "${APPIMAGE_SOURCE}" != *.AppImage ]]; then
  echo "Error: expected an .AppImage file"
  exit 1
fi

BIN_DIR="${XDG_BIN_HOME:-${HOME}/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-${HOME}/.local/share}"
APPLICATIONS_DIR="${DATA_DIR}/applications"
ICON_DIR="${DATA_DIR}/icons/hicolor/256x256/apps"

TARGET_APPIMAGE="${BIN_DIR}/${APP_NAME}.AppImage"
DESKTOP_FILE="${APPLICATIONS_DIR}/${APP_NAME}.desktop"
ICON_FILE="${ICON_DIR}/${APP_NAME}.png"

mkdir -p "${BIN_DIR}" "${APPLICATIONS_DIR}" "${ICON_DIR}"

ACTION="installed"
if [[ -f "${TARGET_APPIMAGE}" ]]; then
  ACTION="updated"
fi

TEMP_APPIMAGE="${TARGET_APPIMAGE}.tmp.$$"
cp "${APPIMAGE_SOURCE}" "${TEMP_APPIMAGE}"
chmod 755 "${TEMP_APPIMAGE}"
mv "${TEMP_APPIMAGE}" "${TARGET_APPIMAGE}"

cat > "${DESKTOP_FILE}" <<DESKTOP
[Desktop Entry]
Type=Application
Name=49modman
Comment=49modman
Exec=${TARGET_APPIMAGE}
Icon=${ICON_FILE}
Terminal=false
Categories=Game;Utility;
StartupNotify=true
DESKTOP

# Optional icon from the repository, if present where this script is executed from.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ICON="${SCRIPT_DIR}/../src-tauri/icons/icon.png"
if [[ -f "${REPO_ICON}" ]]; then
  cp "${REPO_ICON}" "${ICON_FILE}"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${APPLICATIONS_DIR}" >/dev/null 2>&1 || true
fi

echo "${APP_NAME} ${ACTION} at ${TARGET_APPIMAGE}"
echo "Desktop entry: ${DESKTOP_FILE}"
