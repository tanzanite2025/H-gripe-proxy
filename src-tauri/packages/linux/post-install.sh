#!/bin/bash
LEGACY_DEEPIN_DESKTOP_FILE_NAME="Clash Verge.desktop"
CURRENT_DEEPIN_DESKTOP_FILE="/usr/share/applications/clash-verge.desktop"

ensure_service_sidecars_executable() {
    chmod +x /usr/bin/clash-verge-service-install
    chmod +x /usr/bin/clash-verge-service-uninstall
    chmod +x /usr/bin/clash-verge-service
}

is_deepin() {
    [ "${ID:-}" = "deepin" ]
}

rename_legacy_deepin_desktop_file() {
    # Legacy migration from Clash Verge Rev.
    local package_name desktop_files desktop_file

    package_name="${DPKG_MAINTSCRIPT_PACKAGE:-}"
    if [ -z "$package_name" ]; then
        return 0
    fi

    desktop_files=$(dpkg -L "$package_name" 2>/dev/null | grep "\.desktop$" || true)
    echo "$desktop_files" | while IFS= read -r desktop_file; do
        [ -n "$desktop_file" ] || continue
        if [ "$(basename "$desktop_file")" = "$LEGACY_DEEPIN_DESKTOP_FILE_NAME" ]; then
            echo "Migrating Deepin legacy desktop file"
            mv -vf "$desktop_file" "$CURRENT_DEEPIN_DESKTOP_FILE"
        fi
    done
}

. /etc/os-release
ensure_service_sidecars_executable

if is_deepin; then
    rename_legacy_deepin_desktop_file
fi
