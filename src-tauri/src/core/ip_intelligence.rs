use anyhow::{Context as _, Result, anyhow};
use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use crate::utils::dirs;

const ASN_DATABASE_CANDIDATES: [&str; 1] = ["GeoLite2-ASN.mmdb"];
const CITY_DATABASE_CANDIDATES: [&str; 1] = ["GeoLite2-City.mmdb"];
const DEFAULT_PROVIDER_PROBE_IP: &str = "1.1.1.1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IpIntelligenceProviderKind {
    GeoLite2AsnMmdb,
}

impl<'de> Deserialize<'de> for IpIntelligenceProviderKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let _value = String::deserialize(deserializer)?;
        Ok(Self::GeoLite2AsnMmdb)
    }
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
    pub country_name: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub timezone: Option<String>,
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
    pub country_name: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub timezone: Option<String>,
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
    find_provider_factory(&IpIntelligenceProviderKind::GeoLite2AsnMmdb)?
        .create(config)
        .with_context(|| "failed to build local GeoLite2 MMDB provider".to_string())
}

pub async fn probe_provider(
    config: &IpIntelligenceProviderConfig,
    target_ip: Option<&str>,
) -> IpIntelligenceProviderHealthReport {
    let registration = GEOLITE2_ASN_MMDB_PROVIDER_FACTORY.registration();
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
                country_name: None,
                region: None,
                city: None,
                timezone: None,
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
            country_name: record.country_name,
            region: record.region,
            city: record.city,
            timezone: record.timezone,
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
            country_name: None,
            region: None,
            city: None,
            timezone: None,
            checked_at,
        },
    }
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
    asn_database_path: PathBuf,
    city_database_path: PathBuf,
}

struct GeoLite2AsnMmdbProviderFactory;

impl GeoLite2AsnMmdbProvider {
    fn new(_config: &IpIntelligenceProviderConfig) -> Result<Self> {
        Ok(Self {
            asn_database_path: resolve_required_database_path(&ASN_DATABASE_CANDIDATES)?,
            city_database_path: resolve_required_database_path(&CITY_DATABASE_CANDIDATES)?,
        })
    }

    fn asn_reader(&self) -> Result<maxminddb::Reader<Vec<u8>>> {
        maxminddb::Reader::open_readfile(&self.asn_database_path)
            .with_context(|| format!("failed to open ASN database at {}", self.asn_database_path.display()))
    }

    fn city_reader(&self) -> Result<maxminddb::Reader<Vec<u8>>> {
        maxminddb::Reader::open_readfile(&self.city_database_path)
            .with_context(|| format!("failed to open city database at {}", self.city_database_path.display()))
    }
}

impl IpIntelligenceProviderFactory for GeoLite2AsnMmdbProviderFactory {
    fn kind(&self) -> IpIntelligenceProviderKind {
        IpIntelligenceProviderKind::GeoLite2AsnMmdb
    }

    fn registration(&self) -> IpIntelligenceProviderRegistration {
        IpIntelligenceProviderRegistration {
            kind: self.kind(),
            label: "GeoLite2 Local MMDB".to_string(),
            transport: IpIntelligenceProviderTransport::LocalMmdb,
            availability: IpIntelligenceProviderAvailability::Ready,
            description:
                "Read ASN, country, city, and timezone data only from bundled GeoLite2-ASN.mmdb and GeoLite2-City.mmdb."
                    .to_string(),
            fields: Vec::new(),
            default_database_candidates: ASN_DATABASE_CANDIDATES
                .iter()
                .chain(CITY_DATABASE_CANDIDATES.iter())
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
        "GeoLite2 Local MMDB"
    }

    async fn lookup(&self, ip: &str) -> Result<IpIntelligenceRecord> {
        let ip_addr: IpAddr = ip.parse().with_context(|| format!("invalid IP address: {ip}"))?;
        let asn_reader = self.asn_reader()?;
        let asn_lookup = asn_reader
            .lookup(ip_addr)
            .with_context(|| format!("failed to query ASN database for {ip}"))?;
        let asn_record = asn_lookup
            .decode::<maxminddb::geoip2::Asn<'_>>()
            .with_context(|| format!("failed to decode ASN database record for {ip}"))?;
        let asn = asn_record.as_ref().and_then(|item| item.autonomous_system_number);
        let asn_organization = asn_record
            .as_ref()
            .and_then(|item| item.autonomous_system_organization)
            .map(str::to_string);

        let city_reader = self.city_reader()?;
        let city_lookup = city_reader
            .lookup(ip_addr)
            .with_context(|| format!("failed to query city database for {ip}"))?;
        let city_record = city_lookup
            .decode::<maxminddb::geoip2::City<'_>>()
            .with_context(|| format!("failed to decode city database record for {ip}"))?;
        let (country_code, country_name, region, city, timezone) = city_record
            .map(|city_record| {
                let country_code = city_record
                    .country
                    .iso_code
                    .or(city_record.registered_country.iso_code)
                    .map(str::to_ascii_uppercase);
                let country_name = city_record
                    .country
                    .names
                    .english
                    .or(city_record.country.names.simplified_chinese)
                    .map(str::to_string);
                let region = city_record
                    .subdivisions
                    .first()
                    .and_then(|subdivision| subdivision.names.english.or(subdivision.names.simplified_chinese))
                    .map(str::to_string);
                let city = city_record
                    .city
                    .names
                    .english
                    .or(city_record.city.names.simplified_chinese)
                    .map(str::to_string);
                let timezone = city_record.location.time_zone.map(str::to_string);
                (country_code, country_name, region, city, timezone)
            })
            .unwrap_or((None, None, None, None, None));

        Ok(IpIntelligenceRecord {
            asn,
            asn_organization,
            country_code,
            country_name,
            region,
            city,
            timezone,
            provider_label: self.label().to_string(),
        })
    }
}

static GEOLITE2_ASN_MMDB_PROVIDER_FACTORY: GeoLite2AsnMmdbProviderFactory = GeoLite2AsnMmdbProviderFactory;

fn provider_factories() -> [&'static dyn IpIntelligenceProviderFactory; 1] {
    [&GEOLITE2_ASN_MMDB_PROVIDER_FACTORY]
}

fn find_provider_factory(kind: &IpIntelligenceProviderKind) -> Result<&'static dyn IpIntelligenceProviderFactory> {
    provider_factories()
        .into_iter()
        .find(|factory| factory.kind() == *kind)
        .ok_or_else(|| anyhow!("provider {:?} is not registered", kind))
}

fn resolve_first_existing_database_path(candidates: &[&str]) -> Option<PathBuf> {
    candidates
        .iter()
        .find_map(|filename| resolve_from_standard_dirs(filename))
}

fn resolve_required_database_path(candidates: &[&str]) -> Result<PathBuf> {
    resolve_first_existing_database_path(candidates).ok_or_else(|| {
        anyhow!(
            "unable to locate required local MMDB database; looked for [{}] in app home/resources",
            candidates.join(", "),
        )
    })
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
        let registrations = vec![GEOLITE2_ASN_MMDB_PROVIDER_FACTORY.registration()];

        assert_eq!(registrations.len(), 1);
        assert_eq!(registrations[0].kind, IpIntelligenceProviderKind::GeoLite2AsnMmdb);
        assert_eq!(registrations[0].transport, IpIntelligenceProviderTransport::LocalMmdb);
        assert_eq!(registrations[0].availability, IpIntelligenceProviderAvailability::Ready);
        assert!(registrations[0].fields.is_empty());
    }

    #[test]
    fn test_default_candidates_only_use_bundled_geolite_files() {
        assert_eq!(ASN_DATABASE_CANDIDATES, ["GeoLite2-ASN.mmdb"]);
        assert_eq!(CITY_DATABASE_CANDIDATES, ["GeoLite2-City.mmdb"]);
    }

    #[test]
    fn test_legacy_provider_kind_deserializes_to_local_geolite2() {
        let parsed: IpIntelligenceProviderKind = serde_json::from_str(r#""ipinfoHttpApi""#).unwrap();

        assert_eq!(parsed, IpIntelligenceProviderKind::GeoLite2AsnMmdb);
    }
}
