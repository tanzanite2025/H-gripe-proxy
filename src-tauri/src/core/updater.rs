use crate::{config::Config, singleton, utils::dirs};
use anyhow::Result;
use chrono::Utc;
use clash_verge_logging::{Type, logging};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri_plugin_updater::{Update, UpdaterExt as _};

pub struct SilentUpdater {
    update_ready: AtomicBool,
    pending_bytes: RwLock<Option<Vec<u8>>>,
    pending_update: RwLock<Option<Update>>,
    pending_version: RwLock<Option<String>>,
}

singleton!(SilentUpdater, SILENT_UPDATER);

impl SilentUpdater {
    const fn new() -> Self {
        Self {
            update_ready: AtomicBool::new(false),
            pending_bytes: RwLock::new(None),
            pending_update: RwLock::new(None),
            pending_version: RwLock::new(None),
        }
    }

    pub fn is_update_ready(&self) -> bool {
        self.update_ready.load(Ordering::Acquire)
    }
}

// ─── Disk Cache ───────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct UpdateCacheMeta {
    version: String,
    downloaded_at: String,
    #[serde(default)]
    source_key: Option<String>,
}

impl SilentUpdater {
    fn cache_dir() -> Result<PathBuf> {
        Ok(dirs::app_home_dir()?.join("update_cache"))
    }

    fn current_cache_source_key(app_handle: &tauri::AppHandle) -> String {
        let mut parts = vec![
            format!("identifier={}", app_handle.config().identifier),
            format!("repository={}", env!("CARGO_PKG_REPOSITORY")),
        ];

        if let Some(updater) = app_handle.config().plugins.0.get("updater") {
            if let Some(endpoints) = updater.get("endpoints")
                && let Ok(serialized) = serde_json::to_string(endpoints)
            {
                parts.push(format!("endpoints={serialized}"));
            }

            if let Some(pubkey) = updater.get("pubkey").and_then(|value| value.as_str()) {
                parts.push(format!("pubkey={pubkey}"));
            }
        }

        parts.join("\n")
    }

    fn write_cache(bytes: &[u8], version: &str, app_handle: &tauri::AppHandle) -> Result<()> {
        let cache_dir = Self::cache_dir()?;
        std::fs::create_dir_all(&cache_dir)?;

        let bin_path = cache_dir.join("pending_update.bin");
        std::fs::write(&bin_path, bytes)?;

        let meta = UpdateCacheMeta {
            version: version.to_string(),
            downloaded_at: Utc::now().to_rfc3339(),
            source_key: Some(Self::current_cache_source_key(app_handle)),
        };
        let meta_path = cache_dir.join("pending_update.json");
        std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;

        logging!(
            info,
            Type::System,
            "Update cache written: version={}, size={} bytes",
            version,
            bytes.len()
        );
        Ok(())
    }

    fn read_cache_bytes() -> Result<Vec<u8>> {
        let bin_path = Self::cache_dir()?.join("pending_update.bin");
        Ok(std::fs::read(bin_path)?)
    }

    fn read_cache_meta() -> Result<UpdateCacheMeta> {
        let meta_path = Self::cache_dir()?.join("pending_update.json");
        let content = std::fs::read_to_string(meta_path)?;
        Ok(serde_json::from_str(&content)?)
    }

    fn delete_cache() {
        if let Ok(cache_dir) = Self::cache_dir()
            && cache_dir.exists()
        {
            if let Err(e) = std::fs::remove_dir_all(&cache_dir) {
                logging!(warn, Type::System, "Failed to delete update cache: {e}");
            } else {
                logging!(info, Type::System, "Update cache deleted");
            }
        }
    }
}

// ─── Version Comparison ───────────────────────────────────────────────────────

/// Returns true if version `a` <= version `b` using semver-like comparison.
/// Strips leading 'v', splits on '.', handles pre-release suffixes.
fn version_lte(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split('.')
            .filter_map(|part| {
                let numeric = part.split('-').next().unwrap_or("0");
                numeric.parse::<u64>().ok()
            })
            .collect()
    };

    let a_parts = parse(a);
    let b_parts = parse(b);
    let len = a_parts.len().max(b_parts.len());

    for i in 0..len {
        let av = a_parts.get(i).copied().unwrap_or(0);
        let bv = b_parts.get(i).copied().unwrap_or(0);
        if av < bv {
            return true;
        }
        if av > bv {
            return false;
        }
    }
    true // equal
}

fn encode_url_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => encoded.push(byte as char),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

// ─── Startup Install & Cache Management ─────────────────────────────────────

impl SilentUpdater {
    /// Called at app startup. If a cached update exists and is newer than the current version,
    /// attempt to install it immediately (before the main app initializes).
    /// Returns true if install was triggered (app should relaunch), false otherwise.
    pub async fn try_install_on_startup(&self, app_handle: &tauri::AppHandle) -> bool {
        let current_version = env!("CARGO_PKG_VERSION");

        let meta = match Self::read_cache_meta() {
            Ok(meta) => meta,
            Err(_) => return false, // No cache, nothing to do
        };

        let cached_version = &meta.version;
        let current_source_key = Self::current_cache_source_key(app_handle);

        if meta.source_key.as_deref() != Some(current_source_key.as_str()) {
            let source_state = if meta.source_key.is_some() {
                "mismatch"
            } else {
                "missing"
            };

            logging!(
                info,
                Type::System,
                "Update cache source marker is {}, cleaning up stale cache for version {}",
                source_state,
                cached_version
            );
            Self::delete_cache();
            return false;
        }

        if version_lte(cached_version, current_version) {
            logging!(
                info,
                Type::System,
                "Update cache version ({}) <= current ({}), cleaning up",
                cached_version,
                current_version
            );
            Self::delete_cache();
            return false;
        }

        logging!(
            info,
            Type::System,
            "Update cache version ({}) > current ({}), asking user to install",
            cached_version,
            current_version
        );

        // Ask user for confirmation — they can skip and use the app normally.
        // The cache is preserved so next launch will ask again.
        if !Self::ask_user_to_install(app_handle, cached_version).await {
            logging!(info, Type::System, "User skipped update install, starting normally");
            return false;
        }

        // Read cached bytes
        let bytes = match Self::read_cache_bytes() {
            Ok(b) => b,
            Err(e) => {
                logging!(
                    warn,
                    Type::System,
                    "Failed to read cached update bytes: {e}, cleaning up"
                );
                Self::delete_cache();
                return false;
            }
        };

        // Need a fresh Update object from the server to call install().
        // This is a lightweight HTTP request (< 1s), not a re-download.
        let update = match app_handle.updater() {
            Ok(updater) => match updater.check().await {
                Ok(Some(u)) => u,
                Ok(None) => {
                    logging!(
                        info,
                        Type::System,
                        "No update available from server, cache may be stale, cleaning up"
                    );
                    Self::delete_cache();
                    return false;
                }
                Err(e) => {
                    logging!(
                        warn,
                        Type::System,
                        "Failed to check for update at startup: {e}, will retry next launch"
                    );
                    return false; // Keep cache for next attempt
                }
            },
            Err(e) => {
                logging!(
                    warn,
                    Type::System,
                    "Failed to create updater: {e}, will retry next launch"
                );
                return false;
            }
        };

        // Verify the server's version matches the cached version.
        // If server now has a newer version, our cached bytes are stale.
        if update.version != *cached_version {
            logging!(
                info,
                Type::System,
                "Server version ({}) != cached version ({}), cache is stale, cleaning up",
                update.version,
                cached_version
            );
            Self::delete_cache();
            return false;
        }

        let version = update.version.clone();
        logging!(info, Type::System, "Installing cached update v{version} at startup...");

        // Show splash window so user knows the app is updating, not frozen
        Self::show_update_splash(app_handle, &version);

        // install() is sync and may hang (known bug #2558), so run with a timeout.
        // On Windows, NSIS takes over the process so install() may never return — that's OK.
        let install_result = tokio::task::spawn_blocking({
            let bytes = bytes.clone();
            let update = update.clone();
            move || update.install(&bytes)
        });

        let success = match tokio::time::timeout(std::time::Duration::from_secs(30), install_result).await {
            Ok(Ok(Ok(()))) => {
                logging!(info, Type::System, "Update v{version} install triggered at startup");
                Self::delete_cache();
                true
            }
            Ok(Ok(Err(e))) => {
                logging!(
                    warn,
                    Type::System,
                    "Startup install failed: {e}, will retry next launch"
                );
                false
            }
            Ok(Err(e)) => {
                logging!(
                    warn,
                    Type::System,
                    "Startup install task panicked: {e}, will retry next launch"
                );
                false
            }
            Err(_) => {
                logging!(
                    warn,
                    Type::System,
                    "Startup install timed out (30s), will retry next launch"
                );
                false
            }
        };

        // Close splash window if install failed and app continues normally
        if !success {
            Self::close_update_splash(app_handle);
        }

        success
    }
}

// ─── User Confirmation Dialog ────────────────────────────────────────────────

impl SilentUpdater {
    /// Show a native dialog asking the user to install or skip the update.
    /// Returns true if user chose to install, false if they chose to skip.
    async fn ask_user_to_install(app_handle: &tauri::AppHandle, version: &str) -> bool {
        use tauri_plugin_dialog::{DialogExt as _, MessageDialogButtons, MessageDialogKind};

        let title = clash_verge_i18n::t!("notifications.updateReady.title");
        let body = clash_verge_i18n::t!("notifications.updateReady.body").replace("{version}", version);
        let install_now = clash_verge_i18n::t!("notifications.updateReady.installNow").into_owned();
        let later = clash_verge_i18n::t!("notifications.updateReady.later").into_owned();

        let (tx, rx) = tokio::sync::oneshot::channel();

        app_handle
            .dialog()
            .message(body)
            .title(title)
            .buttons(MessageDialogButtons::OkCancelCustom(install_now, later))
            .kind(MessageDialogKind::Info)
            .show(move |confirmed| {
                let _ = tx.send(confirmed);
            });

        rx.await.unwrap_or(false)
    }
}

// ─── Update Splash Window ────────────────────────────────────────────────────

impl SilentUpdater {
    /// Show a small centered splash window indicating update is being installed.
    fn show_update_splash(app_handle: &tauri::AppHandle, version: &str) {
        use tauri::{WebviewUrl, WebviewWindowBuilder};

        let splash_url = format!("update-splash.html?version={}", encode_url_component(version));
        match WebviewWindowBuilder::new(app_handle, "update-splash", WebviewUrl::App(splash_url.into()))
            .title("H-gripe-proxy - Updating")
            .inner_size(300.0, 180.0)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .closable(false)
            .decorations(false)
            .center()
            .always_on_top(true)
            .visible(true)
            .build()
        {
            Ok(_) => {}
            Err(e) => {
                logging!(warn, Type::System, "Failed to create update splash: {e}");
                return;
            }
        };

        logging!(info, Type::System, "Update splash window shown");
    }

    /// Close the update splash window (e.g. after install failure).
    fn close_update_splash(app_handle: &tauri::AppHandle) {
        use tauri::Manager as _;
        if let Some(window) = app_handle.get_webview_window("update-splash") {
            let _ = window.close();
            logging!(info, Type::System, "Update splash window closed");
        }
    }
}

// ─── Background Check and Download ───────────────────────────────────────────

impl SilentUpdater {
    async fn check_and_download(&self, app_handle: &tauri::AppHandle) -> Result<()> {
        let is_portable = *dirs::PORTABLE_FLAG.get().unwrap_or(&false);
        if is_portable {
            logging!(debug, Type::System, "Silent update skipped: portable build");
            return Ok(());
        }

        let auto_check = Config::verge().await.latest_arc().auto_check_update.unwrap_or(true);
        if !auto_check {
            logging!(debug, Type::System, "Silent update skipped: auto_check_update is false");
            return Ok(());
        }

        if self.is_update_ready() {
            logging!(debug, Type::System, "Silent update skipped: update already pending");
            return Ok(());
        }

        logging!(info, Type::System, "Silent updater: checking for updates...");

        let updater = app_handle.updater()?;
        let update = match updater.check().await {
            Ok(Some(update)) => update,
            Ok(None) => {
                logging!(info, Type::System, "Silent updater: no update available");
                return Ok(());
            }
            Err(e) => {
                logging!(warn, Type::System, "Silent updater: check failed: {e}");
                return Err(e.into());
            }
        };

        let version = update.version.clone();
        logging!(info, Type::System, "Silent updater: update available: v{version}");

        if let Some(body) = &update.body
            && body.to_lowercase().contains("break change")
        {
            logging!(
                info,
                Type::System,
                "Silent updater: breaking change detected in v{version}, notifying frontend"
            );
            super::handle::Handle::notice_message(
                "info",
                format!("New version v{version} contains breaking changes. Please update manually."),
            );
            return Ok(());
        }

        logging!(info, Type::System, "Silent updater: downloading v{version}...");
        let bytes = update
            .download(
                |chunk_len, content_len| {
                    logging!(
                        debug,
                        Type::System,
                        "Silent updater download progress: chunk={chunk_len}, total={content_len:?}"
                    );
                },
                || {
                    logging!(info, Type::System, "Silent updater: download complete");
                },
            )
            .await?;

        if let Err(e) = Self::write_cache(&bytes, &version, app_handle) {
            logging!(warn, Type::System, "Silent updater: failed to write cache: {e}");
        }

        *self.pending_bytes.write() = Some(bytes);
        *self.pending_update.write() = Some(update);
        *self.pending_version.write() = Some(version.clone());
        self.update_ready.store(true, Ordering::Release);

        logging!(
            info,
            Type::System,
            "Silent updater: v{version} ready for startup install on next launch"
        );
        Ok(())
    }

    pub async fn start_background_check(&self, app_handle: tauri::AppHandle) {
        logging!(info, Type::System, "Silent updater: background task started");

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        loop {
            if let Err(e) = self.check_and_download(&app_handle).await {
                logging!(warn, Type::System, "Silent updater: cycle error: {e}");
            }

            tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)).await;
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ─── version_lte tests ──────────────────────────────────────────────────

    #[test]
    fn test_version_equal() {
        assert!(version_lte("2.4.7", "2.4.7"));
    }

    #[test]
    fn test_version_less() {
        assert!(version_lte("2.4.7", "2.4.8"));
        assert!(version_lte("2.4.7", "2.5.0"));
        assert!(version_lte("2.4.7", "3.0.0"));
    }

    #[test]
    fn test_version_greater() {
        assert!(!version_lte("2.4.8", "2.4.7"));
        assert!(!version_lte("2.5.0", "2.4.7"));
        assert!(!version_lte("3.0.0", "2.4.7"));
    }

    #[test]
    fn test_version_with_v_prefix() {
        assert!(version_lte("v2.4.7", "2.4.8"));
        assert!(version_lte("2.4.7", "v2.4.8"));
        assert!(version_lte("v2.4.7", "v2.4.8"));
    }

    #[test]
    fn test_version_with_prerelease() {
        // "2.4.8-alpha" → numeric part is still "2.4.8"
        assert!(version_lte("2.4.7", "2.4.8-alpha"));
        assert!(version_lte("2.4.8-alpha", "2.4.8"));
        // Both have same numeric part, so equal → true
        assert!(version_lte("2.4.8-alpha", "2.4.8-beta"));
    }

    #[test]
    fn test_version_different_lengths() {
        assert!(version_lte("2.4", "2.4.1"));
        assert!(!version_lte("2.4.1", "2.4"));
        assert!(version_lte("2.4.0", "2.4"));
    }

    // ─── Cache metadata tests ───────────────────────────────────────────────

    #[test]
    fn test_cache_meta_serialize_roundtrip() {
        let meta = UpdateCacheMeta {
            version: "2.5.0".to_string(),
            downloaded_at: "2026-03-31T00:00:00Z".to_string(),
            source_key: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: UpdateCacheMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, "2.5.0");
        assert_eq!(parsed.downloaded_at, "2026-03-31T00:00:00Z");
    }

    #[test]
    fn test_cache_meta_invalid_json() {
        let result = serde_json::from_str::<UpdateCacheMeta>("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_meta_missing_required_field() {
        let result = serde_json::from_str::<UpdateCacheMeta>(r#"{"version":"2.5.0"}"#);
        assert!(result.is_err()); // missing downloaded_at
    }
}
