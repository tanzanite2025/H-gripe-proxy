//! In-process proxy/rule provider refresh + proxy-provider health checks.
//!
//! The kernel ([`learn_gripe`]) never owns provider data: rule providers are
//! consumed from local files by the app's [`crate::core::rule_engine`], and
//! proxy providers are parsed from local files by
//! [`crate::core::runtime_snapshot`]. The former Mihomo controller
//! `update`/`healthcheck` calls only re-fetched the remote list into the local
//! file (update) or probed the provider's nodes (healthcheck) — both are
//! app-layer concerns identical in shape to the geo update (see
//! [`crate::core::geo_update`]): download the upstream list, validate it, and
//! atomically replace the local file the rule engine / snapshot loads from. The
//! running kernel re-reads providers when the core restarts (see
//! [`crate::core::manager::CoreManager::update_proxy_provider`]).

use crate::config::Config;
use crate::core::runtime_snapshot;
use crate::utils::network::{NetworkManager, ProxyType};
use anyhow::{Context as _, Result, anyhow, bail};
use learn_gripe::{OutboundMode, ProxyEntry};
use serde_yaml_ng::{Mapping, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

const DOWNLOAD_TIMEOUT_SECS: u64 = 60;
/// Default health-check probe URL when a proxy provider configures none. Mirrors
/// mihomo's default provider health-check target.
const DEFAULT_HEALTHCHECK_URL: &str = "https://www.gstatic.com/generate_204";
/// Per-node health-check probe timeout.
const HEALTHCHECK_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProviderKind {
    Proxy,
    Rule,
}

impl ProviderKind {
    fn config_key(self) -> &'static str {
        match self {
            ProviderKind::Proxy => "proxy-providers",
            ProviderKind::Rule => "rule-providers",
        }
    }

    fn label(self) -> &'static str {
        match self {
            ProviderKind::Proxy => "proxy",
            ProviderKind::Rule => "rule",
        }
    }
}

/// Refresh a proxy provider's local file from its remote URL. HTTP providers
/// are downloaded, validated, and atomically swapped in; file/inline providers
/// have nothing remote to fetch and succeed as a no-op (the on-disk file or
/// inline payload is already authoritative).
pub async fn update_proxy_provider(name: &str) -> Result<()> {
    let provider = provider_config(ProviderKind::Proxy, name).await?;
    update_provider_file(ProviderKind::Proxy, name, &provider).await
}

/// Refresh a rule provider's local file from its remote URL (see
/// [`update_proxy_provider`]).
pub async fn update_rule_provider(name: &str) -> Result<()> {
    let provider = provider_config(ProviderKind::Rule, name).await?;
    update_provider_file(ProviderKind::Rule, name, &provider).await
}

/// Probe every measurable node of a proxy provider in process and persist the
/// per-node delay, replacing the Mihomo controller
/// `/providers/proxies/{name}/healthcheck` call. Returns the number of nodes
/// probed. Errors only when the provider is missing or has no measurable nodes.
pub async fn healthcheck_proxy_provider(name: &str) -> Result<usize> {
    let provider = provider_config(ProviderKind::Proxy, name).await?;
    let test_url = healthcheck_url(&provider);

    let nodes = provider_proxy_outbounds(&provider);
    if nodes.is_empty() {
        bail!("proxy provider {name:?} has no measurable nodes to health-check");
    }

    let mut probes = tokio::task::JoinSet::new();
    for (node_name, mode) in nodes {
        let test_url = test_url.clone();
        probes.spawn(async move {
            let delay = learn_gripe::measure_delay(&mode, &test_url, HEALTHCHECK_TIMEOUT)
                .await
                .unwrap_or(0);
            (node_name, delay)
        });
    }

    let mut probed = 0usize;
    while let Some(joined) = probes.join_next().await {
        if let Ok((node_name, delay)) = joined {
            // Keyed by provider name so the snapshot's per-provider node list
            // (matched on node name) picks the delay up, mirroring group probes.
            runtime_snapshot::record_and_persist_runtime_proxy_delay(name, &node_name, delay, &test_url);
            probed += 1;
        }
    }
    Ok(probed)
}

/// Look up a single provider mapping from the active runtime config.
async fn provider_config(kind: ProviderKind, name: &str) -> Result<Mapping> {
    let runtime = Config::runtime().await.latest_arc();
    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow!("no runtime config available for provider update"))?;
    config
        .get(kind.config_key())
        .and_then(Value::as_mapping)
        .and_then(|map| map.get(name))
        .and_then(Value::as_mapping)
        .cloned()
        .ok_or_else(|| anyhow!("{} provider {name:?} not found in runtime config", kind.label()))
}

async fn update_provider_file(kind: ProviderKind, name: &str, provider: &Mapping) -> Result<()> {
    let provider_type = string_field(provider, "type")
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    // Only HTTP providers have a remote list to re-fetch; file/inline providers
    // are already authoritative on disk, so an update is a successful no-op.
    if provider_type != "http" {
        return Ok(());
    }

    let url = string_field(provider, "url")
        .ok_or_else(|| anyhow!("{} provider {name:?} is type http but has no url", kind.label()))?;
    let path = provider_path(provider)
        .ok_or_else(|| anyhow!("{} provider {name:?} is type http but has no local path", kind.label()))?;

    let bytes = download(&url)
        .await
        .with_context(|| format!("failed to download {} provider {name:?} from {url}", kind.label()))?;
    validate(kind, provider, &bytes)
        .with_context(|| format!("downloaded {} provider {name:?} failed validation", kind.label()))?;
    commit(&path, &bytes).with_context(|| {
        format!(
            "failed to install {} provider {name:?} at {}",
            kind.label(),
            path.display()
        )
    })?;
    Ok(())
}

/// Fetch a provider list. The download is attempted through the local proxy
/// first (mirroring mihomo, which fetched providers through its own tunnel) and
/// falls back to a direct connection when the proxy is unavailable.
async fn download(url: &str) -> Result<Vec<u8>> {
    match download_with_proxy(url, ProxyType::Localhost).await {
        Ok(bytes) => Ok(bytes),
        Err(proxied_err) => match download_with_proxy(url, ProxyType::None).await {
            Ok(bytes) => Ok(bytes),
            Err(direct_err) => Err(direct_err.context(format!("proxied download also failed: {proxied_err}"))),
        },
    }
}

async fn download_with_proxy(url: &str, proxy_type: ProxyType) -> Result<Vec<u8>> {
    let client = NetworkManager::new()
        .create_request(proxy_type, Some(DOWNLOAD_TIMEOUT_SECS), None, false)
        .await
        .context("failed to build provider download client")?;
    let response = client
        .get(url)
        .send()
        .await
        .context("provider download request failed")?;
    if !response.status().is_success() {
        bail!("provider download returned HTTP {}", response.status());
    }
    let bytes = response
        .bytes()
        .await
        .context("failed to read provider download body")?;
    if bytes.is_empty() {
        bail!("provider download returned an empty body");
    }
    Ok(bytes.to_vec())
}

/// Validate the downloaded provider list before it is allowed to replace a
/// working file. A list the app could not parse a single entry from is rejected
/// so a bad fetch never blanks out a provider.
fn validate(kind: ProviderKind, provider: &Mapping, bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes).context("provider file is not valid UTF-8")?;
    match kind {
        ProviderKind::Proxy => {
            if proxy_provider_entries(text).is_empty() {
                bail!("no proxies found in downloaded proxy provider");
            }
        }
        ProviderKind::Rule => {
            let is_text = string_field(provider, "format").is_some_and(|format| format.eq_ignore_ascii_case("text"));
            if rule_provider_items(text, is_text).is_empty() {
                bail!("no rules found in downloaded rule provider");
            }
        }
    }
    Ok(())
}

/// Atomically replace `target` with `bytes` via a sibling temp file, so a
/// partial write never clobbers the working provider file.
fn commit(target: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create provider dir at {}", parent.display()))?;
    }
    let tmp = temp_target(target);
    std::fs::write(&tmp, bytes).with_context(|| format!("failed to write temp provider file at {}", tmp.display()))?;
    std::fs::rename(&tmp, target).with_context(|| {
        let _ = std::fs::remove_file(&tmp);
        format!("failed to replace provider file at {}", target.display())
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

/// Resolve a provider's local `path`, relative paths joined onto the app home —
/// the same resolution [`crate::core::runtime_snapshot`] uses to read it back.
fn provider_path(provider: &Mapping) -> Option<PathBuf> {
    let path_str = string_field(provider, "path")?;
    let path = Path::new(&path_str);
    if path.is_absolute() {
        Some(path.to_path_buf())
    } else {
        let app_home = crate::utils::dirs::app_home_dir().ok()?;
        Some(app_home.join(path))
    }
}

fn healthcheck_url(provider: &Mapping) -> String {
    provider
        .get("health-check")
        .and_then(|hc| hc.get("url"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .unwrap_or(DEFAULT_HEALTHCHECK_URL)
        .to_string()
}

/// Resolve a proxy provider's nodes (inline payload or on-disk file) to
/// `(name, outbound)` pairs that can be dialed for a delay probe. Nodes whose
/// protocol the kernel cannot dial (groups, unsupported types) are skipped.
fn provider_proxy_outbounds(provider: &Mapping) -> Vec<(String, OutboundMode)> {
    let mut nodes = Vec::new();
    for entry in load_provider_proxy_entries(provider) {
        let Some(node_name) = entry.get("name").and_then(Value::as_str).map(str::to_string) else {
            continue;
        };
        let Ok(parsed) = serde_yaml_ng::from_value::<ProxyEntry>(entry) else {
            continue;
        };
        if let Ok(mode) = OutboundMode::from_proxy(&parsed) {
            nodes.push((node_name, mode));
        }
    }
    nodes
}

/// A proxy provider's entries, from an inline `payload` or the on-disk file.
fn load_provider_proxy_entries(provider: &Mapping) -> Vec<Value> {
    if let Some(payload) = provider.get("payload").and_then(Value::as_sequence) {
        return payload.clone();
    }
    let Some(path) = provider_path(provider) else {
        return Vec::new();
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => proxy_provider_entries(&content),
        Err(_) => Vec::new(),
    }
}

/// Named proxy entries from a proxy-provider file, accepting either a top-level
/// `proxies:` mapping key or a bare sequence (the two shapes
/// [`crate::core::runtime_snapshot`] reads back).
fn proxy_provider_entries(content: &str) -> Vec<Value> {
    let Ok(value) = serde_yaml_ng::from_str::<Value>(content) else {
        return Vec::new();
    };
    let sequence = value
        .get("proxies")
        .and_then(Value::as_sequence)
        .or_else(|| value.as_sequence());
    sequence
        .map(|items| {
            items
                .iter()
                .filter(|item| item.get("name").and_then(Value::as_str).is_some())
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Meaningful rule entries from a rule-provider file, mirroring the rule
/// engine's parsing: YAML `payload:`/bare list, or line-based text, with blank
/// and comment lines dropped.
fn rule_provider_items(content: &str, is_text: bool) -> Vec<String> {
    let items = if is_text {
        content.lines().map(str::to_owned).collect()
    } else {
        parse_rule_provider_yaml(content)
    };
    items
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty() && !item.starts_with('#'))
        .collect()
}

fn parse_rule_provider_yaml(content: &str) -> Vec<String> {
    #[derive(serde::Deserialize)]
    struct RuleProviderFile {
        payload: Vec<String>,
    }
    if let Ok(file) = serde_yaml_ng::from_str::<RuleProviderFile>(content) {
        return file.payload;
    }
    if let Ok(payload) = serde_yaml_ng::from_str::<Vec<String>>(content) {
        return payload;
    }
    content.lines().map(str::to_owned).collect()
}

fn string_field(map: &Mapping, key: &str) -> Option<String> {
    map.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_provider_entries_reads_proxies_key() {
        let content = "proxies:\n  - {name: a, type: ss, server: 1.2.3.4, port: 8388}\n  - {name: b, type: ss, server: 1.2.3.5, port: 8388}\n";
        let entries = proxy_provider_entries(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].get("name").and_then(Value::as_str), Some("a"));
    }

    #[test]
    fn proxy_provider_entries_reads_bare_sequence() {
        let content = "- {name: a, type: ss, server: 1.2.3.4, port: 8388}\n";
        assert_eq!(proxy_provider_entries(content).len(), 1);
    }

    #[test]
    fn proxy_provider_entries_skips_unnamed_and_invalid() {
        // Unnamed entries are dropped; non-provider YAML yields nothing.
        assert!(proxy_provider_entries("proxies:\n  - {type: ss}\n").is_empty());
        assert!(proxy_provider_entries("not: a provider").is_empty());
        assert!(proxy_provider_entries(": : not yaml").is_empty());
    }

    #[test]
    fn rule_provider_items_parses_payload_yaml() {
        let content = "payload:\n  - 'DOMAIN-SUFFIX,example.com'\n  - 'DOMAIN,test.com'\n";
        let items = rule_provider_items(content, false);
        assert_eq!(items, vec!["DOMAIN-SUFFIX,example.com", "DOMAIN,test.com"]);
    }

    #[test]
    fn rule_provider_items_parses_text_dropping_comments_and_blanks() {
        let content = "# header comment\nexample.com\n\n  test.com  \n";
        let items = rule_provider_items(content, true);
        assert_eq!(items, vec!["example.com", "test.com"]);
    }

    #[test]
    fn validate_rejects_empty_proxy_and_rule_lists() {
        let proxy = Mapping::new();
        let err = validate(ProviderKind::Proxy, &proxy, b"proxies: []").unwrap_err();
        assert!(err.to_string().contains("no proxies"));

        let mut rule = Mapping::new();
        rule.insert(Value::from("format"), Value::from("text"));
        let err = validate(ProviderKind::Rule, &rule, b"# only a comment\n").unwrap_err();
        assert!(err.to_string().contains("no rules"));
    }

    #[test]
    fn validate_accepts_well_formed_lists() {
        let proxy = Mapping::new();
        assert!(validate(ProviderKind::Proxy, &proxy, b"proxies:\n  - {name: a, type: ss}\n").is_ok());

        let rule = Mapping::new();
        assert!(validate(ProviderKind::Rule, &rule, b"payload:\n  - 'DOMAIN,example.com'\n").is_ok());
    }

    #[test]
    fn commit_atomically_replaces_and_leaves_no_temp() {
        let dir = std::env::temp_dir().join(format!("provider-update-commit-{}-{}", std::process::id(), line!()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("provider.yaml");
        std::fs::write(&target, b"old").unwrap();

        commit(&target, b"new contents").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new contents");
        assert!(!temp_target(&target).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn temp_target_is_sibling_of_target() {
        let target = Path::new("/tmp/providers/list.yaml");
        assert_eq!(temp_target(target), Path::new("/tmp/providers/list.yaml.download.tmp"));
    }
}
