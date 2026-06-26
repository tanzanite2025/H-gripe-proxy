//! In-process GeoIP/GeoSite/ASN database updates.
//!
//! The kernel ([`learn_gripe`]) never owns or downloads geo files — it only
//! queries the app's local database through the `GeoLookup` trait. Updating
//! geo data is therefore an app-layer concern: download the upstream files,
//! validate them, and atomically replace the local copies the rule engine
//! loads from. The running kernel re-reads the files when the core restarts
//! (see [`crate::core::manager::CoreManager::update_geo`]).

use crate::config::Config;
use crate::core::rule_geodata::{
    self, ASN_MMDB_CANDIDATES, GEOIP_DAT_CANDIDATES, GEOIP_MMDB_CANDIDATES, GEOSITE_DAT_CANDIDATES, GeoIpData,
    GeoSiteData,
};
use crate::utils::dirs;
use crate::utils::network::{NetworkManager, ProxyType};
use anyhow::{Context as _, Result, anyhow, bail};
use std::path::{Path, PathBuf};

/// Upstream defaults mirror mihomo's built-in `geox-url` values
/// (MetaCubeX `meta-rules-dat`), used when the user has not configured a
/// custom source under the clash `geox-url` key.
const DEFAULT_MMDB_URL: &str = "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/country.mmdb";
const DEFAULT_GEOIP_DAT_URL: &str = "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geoip.dat";
const DEFAULT_GEOSITE_URL: &str = "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat";
const DEFAULT_ASN_URL: &str = "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/GeoLite2-ASN.mmdb";

const DOWNLOAD_TIMEOUT_SECS: u64 = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GeoFormat {
    /// MaxMind binary database (GeoIP `Country.mmdb` / ASN `GeoLite2-ASN.mmdb`).
    Mmdb,
    /// V2Ray-style GeoIP protobuf (`GeoIP.dat`).
    GeoIpDat,
    /// V2Ray-style GeoSite protobuf (`GeoSite.dat`).
    GeoSiteDat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GeoDownload {
    url: String,
    target: PathBuf,
    format: GeoFormat,
}

/// Download and atomically replace the local geo databases the rule engine
/// reads from. Returns the file names that were updated (for logging).
pub async fn update_geo_files() -> Result<Vec<String>> {
    let custom = geox_url_overrides().await;
    let target_dir = dirs::app_home_dir().context("failed to resolve app home dir for geo update")?;
    let downloads = resolve_downloads(&custom, &target_dir);

    let mut updated = Vec::new();
    for download in downloads {
        let bytes = download_geo(&download.url)
            .await
            .with_context(|| format!("failed to download geo database from {}", download.url))?;
        commit_geo(download.format, &download.target, &bytes)
            .with_context(|| format!("failed to install geo database at {}", download.target.display()))?;
        if let Some(name) = download.target.file_name().and_then(|name| name.to_str()) {
            updated.push(name.to_string());
        }
    }

    if updated.is_empty() {
        bail!("no geo databases were updated");
    }
    Ok(updated)
}

/// Custom per-kind URLs configured under the clash `geox-url` mapping.
#[derive(Clone, Default, Debug)]
struct GeoxUrlOverrides {
    geo_ip: Option<String>,
    mmdb: Option<String>,
    asn: Option<String>,
    geo_site: Option<String>,
}

async fn geox_url_overrides() -> GeoxUrlOverrides {
    let clash = Config::clash().await.latest_arc();
    let Some(mapping) = clash.0.get("geox-url").and_then(|value| value.as_mapping()) else {
        return GeoxUrlOverrides::default();
    };
    let get = |key: &str| {
        mapping
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    };
    GeoxUrlOverrides {
        geo_ip: get("geo-ip"),
        mmdb: get("mmdb"),
        asn: get("asn"),
        geo_site: get("geo-site"),
    }
}

fn resolve_downloads(custom: &GeoxUrlOverrides, target_dir: &Path) -> Vec<GeoDownload> {
    resolve_downloads_with(custom, target_dir, rule_geodata::resolve_first_existing)
}

/// Pure resolution shared by [`resolve_downloads`]. `resolve_existing` probes
/// the standard data dirs for a pre-existing file (injected so the logic is
/// testable without an initialized app handle).
fn resolve_downloads_with(
    custom: &GeoxUrlOverrides,
    target_dir: &Path,
    resolve_existing: impl Fn(&[&str]) -> Option<PathBuf>,
) -> Vec<GeoDownload> {
    let mut downloads = Vec::with_capacity(3);

    // GeoIP: the rule engine prefers an MMDB and falls back to a `.dat`. Update
    // whichever representation is already present so the engine keeps loading
    // from the same path; default to MMDB for a fresh install.
    if let Some(target) = resolve_existing(GEOIP_MMDB_CANDIDATES) {
        downloads.push(GeoDownload {
            url: custom.mmdb.clone().unwrap_or_else(|| DEFAULT_MMDB_URL.to_string()),
            target,
            format: GeoFormat::Mmdb,
        });
    } else if let Some(target) = resolve_existing(GEOIP_DAT_CANDIDATES) {
        downloads.push(GeoDownload {
            url: custom
                .geo_ip
                .clone()
                .unwrap_or_else(|| DEFAULT_GEOIP_DAT_URL.to_string()),
            target,
            format: GeoFormat::GeoIpDat,
        });
    } else {
        downloads.push(GeoDownload {
            url: custom.mmdb.clone().unwrap_or_else(|| DEFAULT_MMDB_URL.to_string()),
            target: target_dir.join(GEOIP_MMDB_CANDIDATES[0]),
            format: GeoFormat::Mmdb,
        });
    }

    // GeoSite (always a `.dat`).
    let geosite_target =
        resolve_existing(GEOSITE_DAT_CANDIDATES).unwrap_or_else(|| target_dir.join(GEOSITE_DAT_CANDIDATES[0]));
    downloads.push(GeoDownload {
        url: custom
            .geo_site
            .clone()
            .unwrap_or_else(|| DEFAULT_GEOSITE_URL.to_string()),
        target: geosite_target,
        format: GeoFormat::GeoSiteDat,
    });

    // ASN (MMDB).
    let asn_target = resolve_existing(ASN_MMDB_CANDIDATES).unwrap_or_else(|| target_dir.join(ASN_MMDB_CANDIDATES[1]));
    downloads.push(GeoDownload {
        url: custom.asn.clone().unwrap_or_else(|| DEFAULT_ASN_URL.to_string()),
        target: asn_target,
        format: GeoFormat::Mmdb,
    });

    downloads
}

/// Fetch a geo database. The download is attempted through the local proxy
/// first (mirroring mihomo, which fetched geo data through its own tunnel) and
/// falls back to a direct connection when the proxy is unavailable.
async fn download_geo(url: &str) -> Result<Vec<u8>> {
    match download_geo_with_proxy(url, ProxyType::Localhost).await {
        Ok(bytes) => Ok(bytes),
        Err(proxied_err) => match download_geo_with_proxy(url, ProxyType::None).await {
            Ok(bytes) => Ok(bytes),
            Err(direct_err) => Err(direct_err.context(format!("proxied download also failed: {proxied_err}"))),
        },
    }
}

async fn download_geo_with_proxy(url: &str, proxy_type: ProxyType) -> Result<Vec<u8>> {
    let client = NetworkManager::new()
        .create_request(proxy_type, Some(DOWNLOAD_TIMEOUT_SECS), None, false)
        .await
        .context("failed to build geo download client")?;
    let response = client.get(url).send().await.context("geo download request failed")?;
    if !response.status().is_success() {
        bail!("geo download returned HTTP {}", response.status());
    }
    let bytes = response.bytes().await.context("failed to read geo download body")?;
    if bytes.is_empty() {
        bail!("geo download returned an empty body");
    }
    Ok(bytes.to_vec())
}

/// Validate the downloaded bytes for `format`, then atomically replace
/// `target`. A corrupt download never overwrites a working database.
fn commit_geo(format: GeoFormat, target: &Path, bytes: &[u8]) -> Result<()> {
    match format {
        GeoFormat::GeoIpDat => {
            GeoIpData::from_geoip_dat_bytes(bytes).context("downloaded GeoIP dat failed validation")?;
        }
        GeoFormat::GeoSiteDat => {
            GeoSiteData::from_geosite_dat_bytes(bytes).context("downloaded GeoSite dat failed validation")?;
        }
        GeoFormat::Mmdb => {}
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create geo data dir at {}", parent.display()))?;
    }

    let tmp = temp_target(target);
    std::fs::write(&tmp, bytes).with_context(|| format!("failed to write temp geo file at {}", tmp.display()))?;

    if matches!(format, GeoFormat::Mmdb) {
        if let Err(err) = maxminddb::Reader::open_readfile(&tmp) {
            let _ = std::fs::remove_file(&tmp);
            return Err(anyhow!("downloaded MMDB failed validation: {err}"));
        }
    }

    std::fs::rename(&tmp, target).with_context(|| {
        let _ = std::fs::remove_file(&tmp);
        format!("failed to replace geo file at {}", target.display())
    })?;
    Ok(())
}

fn temp_target(target: &Path) -> PathBuf {
    let mut name = target.file_name().map(|name| name.to_os_string()).unwrap_or_default();
    name.push(".download.tmp");
    match target.parent() {
        Some(parent) => parent.join(name),
        None => PathBuf::from(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_downloads_defaults_to_mmdb_for_fresh_install() {
        // Use a directory with no pre-existing geo files so resolution always
        // falls through to the default file names.
        let dir = std::env::temp_dir().join(format!("geo-update-fresh-{}", std::process::id()));
        let custom = GeoxUrlOverrides::default();
        // No pre-existing geo files: resolution falls through to defaults.
        let downloads = resolve_downloads_with(&custom, &dir, |_| None);

        assert_eq!(downloads.len(), 3);
        let geoip = &downloads[0];
        assert_eq!(geoip.format, GeoFormat::Mmdb);
        assert_eq!(geoip.url, DEFAULT_MMDB_URL);
        assert_eq!(geoip.target.file_name().unwrap(), GEOIP_MMDB_CANDIDATES[0]);

        let geosite = &downloads[1];
        assert_eq!(geosite.format, GeoFormat::GeoSiteDat);
        assert_eq!(geosite.url, DEFAULT_GEOSITE_URL);
        assert_eq!(geosite.target.file_name().unwrap(), GEOSITE_DAT_CANDIDATES[0]);

        let asn = &downloads[2];
        assert_eq!(asn.format, GeoFormat::Mmdb);
        assert_eq!(asn.url, DEFAULT_ASN_URL);
        assert_eq!(asn.target.file_name().unwrap(), ASN_MMDB_CANDIDATES[1]);
    }

    #[test]
    fn resolve_downloads_uses_custom_urls() {
        let dir = std::env::temp_dir().join(format!("geo-update-custom-{}", std::process::id()));
        let custom = GeoxUrlOverrides {
            geo_ip: Some("https://example.com/geoip.dat".into()),
            mmdb: Some("https://example.com/country.mmdb".into()),
            asn: Some("https://example.com/asn.mmdb".into()),
            geo_site: Some("https://example.com/geosite.dat".into()),
        };
        let downloads = resolve_downloads_with(&custom, &dir, |_| None);

        // Fresh install prefers MMDB for GeoIP, so the custom `mmdb` URL wins.
        assert_eq!(downloads[0].url, "https://example.com/country.mmdb");
        assert_eq!(downloads[1].url, "https://example.com/geosite.dat");
        assert_eq!(downloads[2].url, "https://example.com/asn.mmdb");
    }

    #[test]
    fn resolve_downloads_reuses_existing_geoip_dat() {
        let dir = std::env::temp_dir().join(format!("geo-update-existing-{}", std::process::id()));
        let custom = GeoxUrlOverrides::default();
        // Simulate an install that already has a GeoIP `.dat` (no MMDB): the
        // engine keeps loading from it, so we refresh that path as a `.dat`.
        let existing = dir.join(GEOIP_DAT_CANDIDATES[0]);
        let downloads = resolve_downloads_with(&custom, &dir, |candidates| {
            (candidates == GEOIP_DAT_CANDIDATES).then(|| existing.clone())
        });

        assert_eq!(downloads[0].format, GeoFormat::GeoIpDat);
        assert_eq!(downloads[0].url, DEFAULT_GEOIP_DAT_URL);
        assert_eq!(downloads[0].target, existing);
    }

    #[test]
    fn commit_geo_rejects_invalid_mmdb_and_keeps_existing() {
        let dir = std::env::temp_dir().join(format!("geo-update-commit-{}-{}", std::process::id(), line!()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("Country.mmdb");
        std::fs::write(&target, b"existing").unwrap();

        let err = commit_geo(GeoFormat::Mmdb, &target, b"not a valid mmdb").unwrap_err();
        assert!(err.to_string().contains("MMDB failed validation"));
        // Original file is untouched and no temp file is left behind.
        assert_eq!(std::fs::read(&target).unwrap(), b"existing");
        assert!(!temp_target(&target).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn commit_geo_rejects_invalid_geosite_dat() {
        let dir = std::env::temp_dir().join(format!("geo-update-geosite-{}-{}", std::process::id(), line!()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("GeoSite.dat");

        let err = commit_geo(GeoFormat::GeoSiteDat, &target, b"\xff\xff\xff\xff not protobuf").unwrap_err();
        assert!(err.to_string().contains("GeoSite dat failed validation"));
        assert!(!target.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn temp_target_is_sibling_of_target() {
        let target = Path::new("/tmp/geo/Country.mmdb");
        assert_eq!(temp_target(target), Path::new("/tmp/geo/Country.mmdb.download.tmp"));
    }
}
