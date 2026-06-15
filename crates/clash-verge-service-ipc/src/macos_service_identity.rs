#![allow(dead_code)]

pub const APP_BUNDLE_ID: &str = "io.github.tanzanite2025.clash-verge-optimized";
pub const SERVICE_BUNDLE_ID: &str = "io.github.tanzanite2025.clash-verge-optimized.service";
pub const SERVICE_DISPLAY_NAME: &str = "Clash Verge Optimized Service";
pub const SERVICE_EXECUTABLE_NAME: &str = "clash-verge-service";

// Legacy migration from Clash Verge Rev.
pub const LEGACY_SERVICE_BUNDLE_ID: &str = "io.github.clash-verge-rev.clash-verge-rev.service";
pub const LEGACY_HELPER_ID: &str = "io.github.clashverge.helper";

pub fn service_bundle_path() -> String {
    format!("/Library/PrivilegedHelperTools/{SERVICE_BUNDLE_ID}.bundle")
}

pub fn legacy_service_bundle_path() -> String {
    format!("/Library/PrivilegedHelperTools/{LEGACY_SERVICE_BUNDLE_ID}.bundle")
}

pub fn service_plist_path() -> String {
    format!("/Library/LaunchDaemons/{SERVICE_BUNDLE_ID}.plist")
}

pub fn legacy_service_plist_path() -> String {
    format!("/Library/LaunchDaemons/{LEGACY_SERVICE_BUNDLE_ID}.plist")
}

pub fn service_binary_path() -> String {
    format!(
        "{}/Contents/MacOS/{SERVICE_EXECUTABLE_NAME}",
        service_bundle_path()
    )
}

pub fn launchctl_system_target() -> String {
    format!("system/{SERVICE_BUNDLE_ID}")
}

pub fn legacy_launchctl_system_target() -> String {
    format!("system/{LEGACY_SERVICE_BUNDLE_ID}")
}

pub fn legacy_helper_binary_path() -> String {
    format!("/Library/PrivilegedHelperTools/{LEGACY_HELPER_ID}")
}

pub fn legacy_helper_plist_path() -> String {
    format!("/Library/LaunchDaemons/{LEGACY_HELPER_ID}.plist")
}

pub fn legacy_helper_system_target() -> String {
    format!("system/{LEGACY_HELPER_ID}")
}
