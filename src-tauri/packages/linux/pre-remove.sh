#!/bin/bash
LEGACY_DEEPIN_DESKTOP_FILE="/usr/share/applications/Clash Verge.desktop"
CURRENT_DEEPIN_DESKTOP_FILE="/usr/share/applications/clash-verge.desktop"

is_deepin() {
    [ "${ID:-}" = "deepin" ]
}

remove_file_if_exists() {
    local file_path="$1"
    local label="$2"

    if [ -f "$file_path" ]; then
        echo "Removing $label"
        rm -vf "$file_path"
    fi
}

cleanup_deepin_desktop_files() {
    # Legacy migration from Clash Verge Rev.
    remove_file_if_exists "$LEGACY_DEEPIN_DESKTOP_FILE" "legacy Deepin desktop file"
    remove_file_if_exists "$CURRENT_DEEPIN_DESKTOP_FILE" "Deepin desktop file"
}

/usr/bin/clash-verge-service-uninstall

. /etc/os-release

if is_deepin; then
    cleanup_deepin_desktop_files
fi

