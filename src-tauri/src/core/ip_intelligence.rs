use anyhow::{Context as _, Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use crate::utils::dirs;

const ASN_DATABASE_CANDIDATES: [&str; 2] = ["GeoLite2-ASN.mmdb", "ASN.mmdb"];
const IPINFO_LITE_API_ENDPOINT: &str = "https://api.ipinfo.io/lite";
const DEFAULT_PROVIDER_PROBE_IP: &str = "1.1.1.1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IpIntelligenceProviderKind {
    GeoLite2AsnMmdb,
    IpinfoHttpApi,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IpIntelligenceProviderTransport {
    LocalMmdb,
    RemoteHttpApi,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IpIntelligenceProviderAvailability {
    Ready,
    Experimental,
    Placeholder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IpIntelligenceProviderConfigFieldKind {
    DatabasePath,
    ApiEndpoint,
    AccessToken,
    Options,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpIntelligenceProviderConfigField {
    pub kind: IpIntelligenceProviderConfigFieldKind,
    pub label: String,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpIntelligenceProviderRegistration {
    pub kind: IpIntelligenceProviderKind,
    pub label: String,
    pub transport: IpIntelligenceProviderTransport,
    pub availability: IpIntelligenceProviderAvailability,
    pub description: String,
    pub fields: Vec<IpIntelligenceProviderConfigField>,
    pub default_database_candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpIntelligenceProviderHealthReport {
    pub provider_kind: IpIntelligenceProviderKind,
    pub provider_label: String,
    pub availability: IpIntelligenceProviderAvailability,
    pub target_ip: String,
    pub healthy: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
    pub asn: Option<String>,
    pub asn_org: Option<String>,
    pub country_code: Option<String>,
    pub checked_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpIntelligenceProviderConfig {
    pub kind: IpIntelligenceProviderKind,
    #[serde(default)]
    pub database_path: Option<String>,
    #[serde(default)]
    pub api_endpoint: Option<String>,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub options: HashMap<String, String>,
}

impl Default for IpIntelligenceProviderConfig {
    fn default() -> Self {
        Self {
            kind: IpIntelligenceProviderKind::GeoLite2AsnMmdb,
            database_path: None,
            api_endpoint: None,
            access_token: None,
            options: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IpIntelligenceRecord {
    pub asn: Option<u32>,
    pub asn_organization: Option<String>,
    pub country_code: Option<String>,
    pub provider_label: String,
}

#[async_trait]
pub trait IpIntelligenceProvider: Send + Sync {
    fn label(&self) -> &'static str;
    async fn lookup(&self, ip: &str) -> Result<IpIntelligenceRecord>;
}

trait IpIntelligenceProviderFactory: Send + Sync {
    fn kind(&self) -> IpIntelligenceProviderKind;
    fn registration(&self) -> IpIntelligenceProviderRegistration;
    fn create(&self, config: &IpIntelligenceProviderConfig) -> Result<Box<dyn IpIntelligenceProvider>>;
}

pub fn build_provider(config: &IpIntelligenceProviderConfig) -> Result<Box<dyn IpIntelligenceProvider>> {
    find_provider_factory(&config.kind)?
        .create(config)
        .with_context(|| format!("failed to build provider {:?}", config.kind))
}

pub fn get_provider_registrations() -> Vec<IpIntelligenceProviderRegistration> {
    provider_factories()
        .iter()
        .map(|factory| factory.registration())
        .collect()
}

pub async fn probe_provider(
    config: &IpIntelligenceProviderConfig,
    target_ip: Option<&str>,
) -> IpIntelligenceProviderHealthReport {
    let registration = get_provider_registration(&config.kind).unwrap_or(IpIntelligenceProviderRegistration {
        kind: config.kind.clone(),
        label: format!("{:?}", config.kind),
        transport: IpIntelligenceProviderTransport::Custom,
        availability: IpIntelligenceProviderAvailability::Placeholder,
        description: "provider registration is unavailable".to_string(),
        fields: Vec::new(),
        default_database_candidates: Vec::new(),
    });
    let target_ip = target_ip
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_PROVIDER_PROBE_IP)
        .to_string();
    let checked_at = SystemTime::now();

    let provider = match build_provider(config) {
        Ok(provider) => provider,
        Err(error) => {
            return IpIntelligenceProviderHealthReport {
                provider_kind: registration.kind,
                provider_label: registration.label,
                availability: registration.availability,
                target_ip,
                healthy: false,
                message: error.to_string(),
                latency_ms: None,
                asn: None,
                asn_org: None,
                country_code: None,
                checked_at,
            };
        }
    };

    let started_at = Instant::now();
    match provider.lookup(&target_ip).await {
        Ok(record) => IpIntelligenceProviderHealthReport {
            provider_kind: registration.kind,
            provider_label: provider.label().to_string(),
            availability: registration.availability,
            target_ip,
            healthy: true,
            message: "provider lookup succeeded".to_string(),
            latency_ms: Some(started_at.elapsed().as_millis() as u64),
            asn: record.asn.map(|asn| format!("AS{asn}")),
            asn_org: record.asn_organization,
            country_code: record.country_code,
            checked_at,
        },
        Err(error) => IpIntelligenceProviderHealthReport {
            provider_kind: registration.kind,
            provider_label: provider.label().to_string(),
            availability: registration.availability,
            target_ip,
            healthy: false,
            message: error.to_string(),
            latency_ms: Some(started_at.elapsed().as_millis() as u64),
            asn: None,
            asn_org: None,
            country_code: None,
            checked_at,
        },
    }
}

pub fn resolve_database_path(config: &IpIntelligenceProviderConfig) -> Option<PathBuf> {
    if let Some(path) = config.database_path.as_deref() {
        let configured_path = PathBuf::from(path);
        if configured_path.exists() {
            return Some(configured_path);
        }

        if configured_path.is_relative()
            && let Ok(app_home) = dirs::app_home_dir()
        {
            let app_home_path = app_home.join(&configured_path);
            if app_home_path.exists() {
                return Some(app_home_path);
            }
        }
    }

    for filename in ASN_DATABASE_CANDIDATES {
        if let Some(path) = resolve_from_standard_dirs(filename) {
            return Some(path);
        }
    }

    None
}

fn resolve_from_standard_dirs(filename: &str) -> Option<PathBuf> {
    for candidate in standard_database_dirs() {
        let path = candidate.join(filename);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn standard_database_dirs() -> Vec<PathBuf> {
    let mut dirs_to_check = Vec::new();

    if let Ok(app_home) = dirs::app_home_dir() {
        dirs_to_check.push(app_home);
    }

    if let Ok(resources_dir) = dirs::app_resources_dir() {
        dirs_to_check.push(resources_dir);
    }

    dirs_to_check
}

struct GeoLite2AsnMmdbProvider {
    database_path: PathBuf,
}

struct GeoLite2AsnMmdbProviderFactory;

struct IpinfoHttpApiProvider {
    client: Client,
    api_endpoint: String,
    access_token: String,
}

struct IpinfoHttpApiProviderFactory;

impl GeoLite2AsnMmdbProvider {
    fn new(config: &IpIntelligenceProviderConfig) -> Result<Self> {
        let database_path = resolve_database_path(config).ok_or_else(|| {
            anyhow!(
                "unable to locate GeoLite2 ASN database; looked for {}",
                ASN_DATABASE_CANDIDATES.join(", ")
            )
        })?;

        Ok(Self { database_path })
    }

    fn reader(&self) -> Result<maxminddb::Reader<Vec<u8>>> {
        maxminddb::Reader::open_readfile(&self.database_path)
            .with_context(|| format!("failed to open ASN database at {}", self.database_path.display()))
    }
}

impl IpIntelligenceProviderFactory for GeoLite2AsnMmdbProviderFactory {
    fn kind(&self) -> IpIntelligenceProviderKind {
        IpIntelligenceProviderKind::GeoLite2AsnMmdb
    }

    fn registration(&self) -> IpIntelligenceProviderRegistration {
        IpIntelligenceProviderRegistration {
            kind: self.kind(),
            label: "GeoLite2 ASN MMDB".to_string(),
            transport: IpIntelligenceProviderTransport::LocalMmdb,
            availability: IpIntelligenceProviderAvailability::Ready,
            description: "Read ASN and organization data from a local MaxMind-compatible ASN database."
                .to_string(),
            fields: vec![IpIntelligenceProviderConfigField {
                kind: IpIntelligenceProviderConfigFieldKind::DatabasePath,
                label: "Database Path".to_string(),
                required: false,
                description:
                    "Optional override path. If omitted, the app searches GeoLite2-ASN.mmdb and ASN.mmdb in the standard app directories."
                        .to_string(),
            }],
            default_database_candidates: ASN_DATABASE_CANDIDATES
                .iter()
                .map(|item| item.to_string())
                .collect(),
        }
    }

    fn create(&self, config: &IpIntelligenceProviderConfig) -> Result<Box<dyn IpIntelligenceProvider>> {
        Ok(Box::new(GeoLite2AsnMmdbProvider::new(config)?))
    }
}

#[async_trait]
impl IpIntelligenceProvider for GeoLite2AsnMmdbProvider {
    fn label(&self) -> &'static str {
        "GeoLite2 ASN MMDB"
    }

    async fn lookup(&self, ip: &str) -> Result<IpIntelligenceRecord> {
        let ip_addr: IpAddr = ip.parse().with_context(|| format!("invalid IP address: {ip}"))?;
        let reader = self.reader()?;
        let lookup = reader
            .lookup(ip_addr)
            .with_context(|| format!("failed to query ASN database for {ip}"))?;
        let result = lookup
            .decode::<maxminddb::geoip2::Asn<'_>>()
            .with_context(|| format!("failed to decode ASN database record for {ip}"))?;

        Ok(IpIntelligenceRecord {
            asn: result.as_ref().and_then(|item| item.autonomous_system_number),
            asn_organization: result
                .as_ref()
                .and_then(|item| item.autonomous_system_organization)
                .map(str::to_string),
            country_code: None,
            provider_label: self.label().to_string(),
        })
    }
}

impl IpinfoHttpApiProvider {
    fn new(config: &IpIntelligenceProviderConfig) -> Result<Self> {
        let access_token = config
            .access_token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .ok_or_else(|| anyhow!("IPinfo HTTP API requires a non-empty access token"))?;

        let api_endpoint = config
            .api_endpoint
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(IPINFO_LITE_API_ENDPOINT)
            .trim_end_matches('/')
            .to_string();

        let timeout_seconds = config
            .options
            .get("timeoutSeconds")
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(10);

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .with_context(|| "failed to build IPinfo HTTP client".to_string())?;

        Ok(Self {
            client,
            api_endpoint,
            access_token,
        })
    }
}

#[derive(Debug, Deserialize)]
struct IpinfoLiteLookupResponse {
    asn: Option<String>,
    as_name: Option<String>,
    country_code: Option<String>,
}

impl IpIntelligenceProviderFactory for IpinfoHttpApiProviderFactory {
    fn kind(&self) -> IpIntelligenceProviderKind {
        IpIntelligenceProviderKind::IpinfoHttpApi
    }

    fn registration(&self) -> IpIntelligenceProviderRegistration {
        IpIntelligenceProviderRegistration {
            kind: self.kind(),
            label: "IPinfo HTTP API".to_string(),
            transport: IpIntelligenceProviderTransport::RemoteHttpApi,
            availability: IpIntelligenceProviderAvailability::Experimental,
            description:
                "Fetch ASN and country metadata from the IPinfo Lite HTTPS API. Implemented for opt-in use, but not selected by default."
                    .to_string(),
            fields: vec![
                IpIntelligenceProviderConfigField {
                    kind: IpIntelligenceProviderConfigFieldKind::ApiEndpoint,
                    label: "API Endpoint".to_string(),
                    required: false,
                    description:
                        "Optional override for the IPinfo endpoint. Leave empty to use the built-in Lite endpoint."
                            .to_string(),
                },
                IpIntelligenceProviderConfigField {
                    kind: IpIntelligenceProviderConfigFieldKind::AccessToken,
                    label: "Access Token".to_string(),
                    required: true,
                    description: "IPinfo API token for authenticated lookup requests.".to_string(),
                },
                IpIntelligenceProviderConfigField {
                    kind: IpIntelligenceProviderConfigFieldKind::Options,
                    label: "Extra Options".to_string(),
                    required: false,
                    description:
                        "Optional request tuning fields. The current adapter recognizes timeoutSeconds if provided."
                            .to_string(),
                },
            ],
            default_database_candidates: Vec::new(),
        }
    }

    fn create(&self, config: &IpIntelligenceProviderConfig) -> Result<Box<dyn IpIntelligenceProvider>> {
        Ok(Box::new(IpinfoHttpApiProvider::new(config)?))
    }
}

#[async_trait]
impl IpIntelligenceProvider for IpinfoHttpApiProvider {
    fn label(&self) -> &'static str {
        "IPinfo HTTP API"
    }

    async fn lookup(&self, ip: &str) -> Result<IpIntelligenceRecord> {
        let _: IpAddr = ip.parse().with_context(|| format!("invalid IP address: {ip}"))?;
        let url = format!("{}/{}", self.api_endpoint, ip);
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .with_context(|| format!("failed to request IPinfo metadata for {ip}"))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let detail = body.trim();
            return Err(anyhow!(
                "IPinfo lookup failed with status {}{}",
                status,
                if detail.is_empty() {
                    String::new()
                } else {
                    format!(": {detail}")
                }
            ));
        }

        let payload = response
            .json::<IpinfoLiteLookupResponse>()
            .await
            .with_context(|| format!("failed to decode IPinfo response for {ip}"))?;

        Ok(IpIntelligenceRecord {
            asn: payload.asn.as_deref().and_then(parse_asn_text),
            asn_organization: payload.as_name,
            country_code: payload.country_code,
            provider_label: self.label().to_string(),
        })
    }
}

static GEOLITE2_ASN_MMDB_PROVIDER_FACTORY: GeoLite2AsnMmdbProviderFactory = GeoLite2AsnMmdbProviderFactory;
static IPINFO_HTTP_API_PROVIDER_FACTORY: IpinfoHttpApiProviderFactory = IpinfoHttpApiProviderFactory;

fn provider_factories() -> [&'static dyn IpIntelligenceProviderFactory; 2] {
    [&GEOLITE2_ASN_MMDB_PROVIDER_FACTORY, &IPINFO_HTTP_API_PROVIDER_FACTORY]
}

fn find_provider_factory(kind: &IpIntelligenceProviderKind) -> Result<&'static dyn IpIntelligenceProviderFactory> {
    provider_factories()
        .into_iter()
        .find(|factory| factory.kind() == *kind)
        .ok_or_else(|| anyhow!("provider {:?} is not registered", kind))
}

fn get_provider_registration(kind: &IpIntelligenceProviderKind) -> Option<IpIntelligenceProviderRegistration> {
    provider_factories()
        .into_iter()
        .find(|factory| factory.kind() == *kind)
        .map(|factory| factory.registration())
}

fn parse_asn_text(value: &str) -> Option<u32> {
    let trimmed = value.trim();
    let normalized = trimmed
        .strip_prefix("AS")
        .or_else(|| trimmed.strip_prefix("as"))
        .unwrap_or(trimmed);

    normalized.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn looks_like_mmdb_path(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mmdb"))
    }

    #[test]
    fn test_default_provider_config_prefers_geolite2_asn_mmdb() {
        let config = IpIntelligenceProviderConfig::default();

        assert_eq!(config.kind, IpIntelligenceProviderKind::GeoLite2AsnMmdb);
        assert!(config.database_path.is_none());
        assert!(config.api_endpoint.is_none());
        assert!(config.access_token.is_none());
        assert!(config.options.is_empty());
    }

    #[test]
    fn test_looks_like_mmdb_path() {
        assert!(looks_like_mmdb_path(Path::new("GeoLite2-ASN.mmdb")));
        assert!(looks_like_mmdb_path(Path::new("ASN.MMDB")));
        assert!(!looks_like_mmdb_path(Path::new("asn.dat")));
    }

    #[test]
    fn test_provider_registry_exposes_registered_providers() {
        let registrations = get_provider_registrations();

        assert_eq!(registrations.len(), 2);
        assert_eq!(registrations[0].kind, IpIntelligenceProviderKind::GeoLite2AsnMmdb);
        assert_eq!(registrations[0].transport, IpIntelligenceProviderTransport::LocalMmdb);
        assert_eq!(registrations[0].availability, IpIntelligenceProviderAvailability::Ready);
        assert!(
            registrations[0]
                .fields
                .iter()
                .any(|field| field.kind == IpIntelligenceProviderConfigFieldKind::DatabasePath)
        );

        assert_eq!(registrations[1].kind, IpIntelligenceProviderKind::IpinfoHttpApi);
        assert_eq!(
            registrations[1].transport,
            IpIntelligenceProviderTransport::RemoteHttpApi
        );
        assert_eq!(
            registrations[1].availability,
            IpIntelligenceProviderAvailability::Experimental
        );
        assert!(
            registrations[1]
                .fields
                .iter()
                .any(|field| field.kind == IpIntelligenceProviderConfigFieldKind::AccessToken)
        );
    }

    #[test]
    fn test_ipinfo_provider_can_be_built_when_token_is_present() {
        let config = IpIntelligenceProviderConfig {
            kind: IpIntelligenceProviderKind::IpinfoHttpApi,
            database_path: None,
            api_endpoint: Some("https://api.ipinfo.io/lite".to_string()),
            access_token: Some("token-placeholder".to_string()),
            options: HashMap::new(),
        };

        let provider = build_provider(&config).unwrap();

        assert_eq!(provider.label(), "IPinfo HTTP API");
    }

    #[test]
    fn test_ipinfo_provider_requires_access_token() {
        let config = IpIntelligenceProviderConfig {
            kind: IpIntelligenceProviderKind::IpinfoHttpApi,
            database_path: None,
            api_endpoint: Some("https://api.ipinfo.io/lite".to_string()),
            access_token: None,
            options: HashMap::new(),
        };

        let error = build_provider(&config).unwrap_err();

        assert!(error.to_string().contains("access token"));
    }

    #[test]
    fn test_parse_asn_text() {
        assert_eq!(parse_asn_text("AS15169"), Some(15169));
        assert_eq!(parse_asn_text("15169"), Some(15169));
        assert_eq!(parse_asn_text(" as7922 "), Some(7922));
        assert_eq!(parse_asn_text("invalid"), None);
    }
}
