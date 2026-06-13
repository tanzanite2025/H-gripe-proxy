#!/bin/bash
LEGACY_DEEPIN_DESKTOP_FILE="Clash Verge.desktop"
CURRENT_DEEPIN_DESKTOP_FILE="/usr/share/applications/clash-verge.desktop"

chmod +x /usr/bin/clash-verge-service-install
chmod +x /usr/bin/clash-verge-service-uninstall
chmod +x /usr/bin/clash-verge-service

. /etc/os-release

if [ "$ID" = "deepin" ]; then
    PACKAGE_NAME="$DPKG_MAINTSCRIPT_PACKAGE"
    DESKTOP_FILES=$(dpkg -L "$PACKAGE_NAME" 2>/dev/null | grep "\.desktop$")
    echo "$DESKTOP_FILES" | while IFS= read -r f; do
        if [ "$(basename "$f")" == "$LEGACY_DEEPIN_DESKTOP_FILE" ]; then
            echo "Fixing deepin legacy desktop file"
            mv -vf "$f" "$CURRENT_DEEPIN_DESKTOP_FILE"
        fi
    done
fi
