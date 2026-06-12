use anyhow::{Context as _, Result, anyhow};
use prost::Message;
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::HashMap,
    net::IpAddr,
    path::{Path, PathBuf},
};

use crate::utils::dirs;

const GEOIP_MMDB_CANDIDATES: &[&str] = &["Country.mmdb", "geoip.metadb", "geoip.db", "GeoLite2-City.mmdb"];
const GEOIP_DAT_CANDIDATES: &[&str] = &["GeoIP.dat"];
const GEOSITE_DAT_CANDIDATES: &[&str] = &["GeoSite.dat"];

#[derive(Clone, Default)]
pub struct RuleGeoData {
    geoip: Option<GeoIpData>,
    geosite: Option<GeoSiteData>,
}

impl RuleGeoData {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn load_default() -> Self {
        Self {
            geoip: GeoIpData::load_default().ok(),
            geosite: GeoSiteData::load_default().ok(),
        }
    }

    #[cfg(test)]
    pub fn from_parts(geoip: Option<GeoIpData>, geosite: Option<GeoSiteData>) -> Self {
        Self { geoip, geosite }
    }

    pub fn geoip_matches(&self, code: &str, ip: IpAddr) -> bool {
        self.geoip.as_ref().is_some_and(|geoip| geoip.matches(code, ip))
    }

    pub fn geosite_matches(&self, code: &str, host: &str) -> bool {
        self.geosite.as_ref().is_some_and(|geosite| geosite.matches(code, host))
    }
}

#[derive(Clone)]
pub struct GeoIpData {
    source: GeoIpSource,
}

#[derive(Clone)]
enum GeoIpSource {
    Mmdb(PathBuf),
    Cidr(HashMap<String, Vec<(IpAddr, u8)>>),
}

impl GeoIpData {
    #[cfg(test)]
    pub fn from_cidr_map(cidrs: HashMap<String, Vec<(IpAddr, u8)>>) -> Self {
        Self {
            source: GeoIpSource::Cidr(cidrs),
        }
    }

    pub fn load_default() -> Result<Self> {
        if let Some(path) = resolve_first_existing(GEOIP_MMDB_CANDIDATES) {
            return Self::from_mmdb_path(path);
        }
        if let Some(path) = resolve_first_existing(GEOIP_DAT_CANDIDATES) {
            return Self::from_geoip_dat_path(path);
        }
        Err(anyhow!("GeoIP data not found"))
    }

    pub fn from_mmdb_path(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        maxminddb::Reader::open_readfile(&path)
            .with_context(|| format!("failed to open GeoIP MMDB at {}", path.display()))?;
        Ok(Self {
            source: GeoIpSource::Mmdb(path),
        })
    }

    pub fn from_geoip_dat_path(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path.as_ref())
            .with_context(|| format!("failed to read GeoIP dat at {}", path.as_ref().display()))?;
        Self::from_geoip_dat_bytes(&bytes)
    }

    pub fn from_geoip_dat_bytes(bytes: &[u8]) -> Result<Self> {
        let list = GeoIpList::decode(bytes).context("failed to decode GeoIP protobuf data")?;
        let mut cidrs: HashMap<String, Vec<(IpAddr, u8)>> = HashMap::new();
        for entry in list.entry {
            let code = normalize_code(&entry.country_code);
            let ranges = entry
                .cidr
                .into_iter()
                .filter_map(|cidr| cidr_to_range(cidr).ok())
                .collect::<Vec<_>>();
            cidrs.insert(code, ranges);
        }
        Ok(Self {
            source: GeoIpSource::Cidr(cidrs),
        })
    }

    fn matches(&self, code: &str, ip: IpAddr) -> bool {
        let (negated, code) = parse_negated_code(code);
        match &self.source {
            GeoIpSource::Mmdb(path) => {
                let matched = mmdb_codes(path, ip).is_ok_and(|codes| codes.iter().any(|item| item == &code));
                matched != negated
            }
            GeoIpSource::Cidr(cidrs) => cidrs.get(&code).is_some_and(|ranges| {
                let matched = ranges.iter().any(|(addr, prefix)| cidr_contains(*addr, *prefix, ip));
                matched != negated
            }),
        }
    }
}

#[derive(Clone)]
pub struct GeoSiteData {
    sites: HashMap<String, GeoSiteMatcher>,
}

impl GeoSiteData {
    #[cfg(test)]
    pub fn from_site_map(sites: HashMap<String, Vec<(GeoSiteDomainType, String)>>) -> Result<Self> {
        let mut mapped = HashMap::new();
        for (code, domains) in sites {
            mapped.insert(
                normalize_code(&code),
                GeoSiteMatcher::new(
                    domains
                        .into_iter()
                        .map(|(kind, value)| GeoSiteDomain {
                            r#type: kind as i32,
                            value,
                        })
                        .collect(),
                )?,
            );
        }
        Ok(Self { sites: mapped })
    }

    pub fn load_default() -> Result<Self> {
        let path = resolve_first_existing(GEOSITE_DAT_CANDIDATES).ok_or_else(|| anyhow!("GeoSite data not found"))?;
        Self::from_geosite_dat_path(path)
    }

    pub fn from_geosite_dat_path(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path.as_ref())
            .with_context(|| format!("failed to read GeoSite dat at {}", path.as_ref().display()))?;
        Self::from_geosite_dat_bytes(&bytes)
    }

    pub fn from_geosite_dat_bytes(bytes: &[u8]) -> Result<Self> {
        let list = GeoSiteList::decode(bytes).context("failed to decode GeoSite protobuf data")?;
        let mut sites = HashMap::new();
        for entry in list.entry {
            sites.insert(normalize_code(&entry.country_code), GeoSiteMatcher::new(entry.domain)?);
        }
        Ok(Self { sites })
    }

    fn matches(&self, code: &str, host: &str) -> bool {
        let (negated, code) = parse_negated_code(code);
        let host = host.to_ascii_lowercase();
        self.sites
            .get(&code)
            .is_some_and(|matcher| matcher.matches(&host) != negated)
    }
}

#[derive(Clone)]
struct GeoSiteMatcher {
    plains: Vec<String>,
    regexes: Vec<Regex>,
    domains: Vec<String>,
    fulls: Vec<String>,
}

impl GeoSiteMatcher {
    fn new(domains: Vec<GeoSiteDomain>) -> Result<Self> {
        let mut matcher = Self {
            plains: Vec::new(),
            regexes: Vec::new(),
            domains: Vec::new(),
            fulls: Vec::new(),
        };

        for domain in domains {
            let value = domain.value.to_ascii_lowercase();
            match GeoSiteDomainType::try_from(domain.r#type).unwrap_or(GeoSiteDomainType::Plain) {
                GeoSiteDomainType::Plain => matcher.plains.push(value),
                GeoSiteDomainType::Regex => matcher
                    .regexes
                    .push(Regex::new(&format!("(?i){}", domain.value)).context("invalid GeoSite regex")?),
                GeoSiteDomainType::Domain => matcher.domains.push(value),
                GeoSiteDomainType::Full => matcher.fulls.push(value),
            }
        }

        Ok(matcher)
    }

    fn matches(&self, host: &str) -> bool {
        self.fulls.iter().any(|full| host == full)
            || self.domains.iter().any(|domain| domain_match(domain, host))
            || self.plains.iter().any(|plain| host.contains(plain))
            || self.regexes.iter().any(|regex| regex.is_match(host))
    }
}

#[derive(Debug, Deserialize)]
struct MmdbCountryRecord<'a> {
    #[serde(default)]
    country: MmdbCountry<'a>,
    #[serde(default)]
    registered_country: MmdbCountry<'a>,
}

#[derive(Debug, Default, Deserialize)]
struct MmdbCountry<'a> {
    iso_code: Option<&'a str>,
}

fn mmdb_codes(path: &Path, ip: IpAddr) -> Result<Vec<String>> {
    let reader = maxminddb::Reader::open_readfile(path)
        .with_context(|| format!("failed to open GeoIP MMDB at {}", path.display()))?;
    match reader.metadata.database_type.as_str() {
        "sing-geoip" => decode_mmdb_string(&reader, ip).map(|code| vec![normalize_code(&code)]),
        "Meta-geoip0" => decode_meta_geoip0(&reader, ip),
        _ => {
            let lookup = reader
                .lookup(ip)
                .with_context(|| format!("failed to query GeoIP MMDB for {ip}"))?;
            let record = lookup
                .decode::<MmdbCountryRecord<'_>>()
                .with_context(|| format!("failed to decode GeoIP MMDB record for {ip}"))?;
            Ok(record
                .into_iter()
                .flat_map(|record| {
                    [record.country.iso_code, record.registered_country.iso_code]
                        .into_iter()
                        .flatten()
                        .map(normalize_code)
                })
                .collect())
        }
    }
}

fn decode_mmdb_string(reader: &maxminddb::Reader<Vec<u8>>, ip: IpAddr) -> Result<String> {
    let lookup = reader
        .lookup(ip)
        .with_context(|| format!("failed to query GeoIP MMDB for {ip}"))?;
    lookup
        .decode::<String>()
        .with_context(|| format!("failed to decode GeoIP string record for {ip}"))?
        .ok_or_else(|| anyhow!("GeoIP MMDB has no record for {ip}"))
}

fn decode_meta_geoip0(reader: &maxminddb::Reader<Vec<u8>>, ip: IpAddr) -> Result<Vec<String>> {
    if let Ok(code) = decode_mmdb_string(reader, ip) {
        return Ok(vec![normalize_code(&code)]);
    }

    let lookup = reader
        .lookup(ip)
        .with_context(|| format!("failed to query GeoIP MMDB for {ip}"))?;
    let codes = lookup
        .decode::<Vec<String>>()
        .with_context(|| format!("failed to decode GeoIP list record for {ip}"))?
        .unwrap_or_default()
        .into_iter()
        .map(|code| normalize_code(&code))
        .collect();
    Ok(codes)
}

fn resolve_first_existing(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().find_map(|candidate| {
        standard_data_dirs()
            .into_iter()
            .map(|dir| dir.join(candidate))
            .find(|path| path.is_file())
    })
}

fn standard_data_dirs() -> Vec<PathBuf> {
    let mut dirs_to_check = Vec::new();
    if let Ok(app_home) = dirs::app_home_dir() {
        dirs_to_check.push(app_home);
    }
    if let Ok(resources_dir) = dirs::app_resources_dir() {
        dirs_to_check.push(resources_dir);
    }
    dirs_to_check
}

fn normalize_code(code: &str) -> String {
    code.trim().to_ascii_lowercase()
}

fn parse_negated_code(code: &str) -> (bool, String) {
    let code = normalize_code(code);
    if let Some(stripped) = code.strip_prefix('!') {
        (true, stripped.to_string())
    } else {
        (false, code)
    }
}

fn domain_match(domain: &str, host: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

fn cidr_to_range(cidr: GeoIpCidr) -> Result<(IpAddr, u8)> {
    let prefix = u8::try_from(cidr.prefix).context("GeoIP CIDR prefix exceeds u8")?;
    let addr = match cidr.ip.as_slice() {
        [a, b, c, d] => IpAddr::from([*a, *b, *c, *d]),
        bytes if bytes.len() == 16 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(bytes);
            IpAddr::from(octets)
        }
        _ => return Err(anyhow!("GeoIP CIDR has invalid IP byte length")),
    };
    let max_prefix = match addr {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    };
    if prefix > max_prefix {
        return Err(anyhow!("GeoIP CIDR prefix {prefix} exceeds maximum {max_prefix}"));
    }
    Ok((addr, prefix))
}

fn cidr_contains(network: IpAddr, prefix_len: u8, ip: IpAddr) -> bool {
    match (network, ip) {
        (IpAddr::V4(net), IpAddr::V4(ip)) => {
            let net = u32::from(net);
            let ip = u32::from(ip);
            let mask = if prefix_len == 0 {
                0
            } else {
                u32::MAX << (32 - prefix_len)
            };
            (net & mask) == (ip & mask)
        }
        (IpAddr::V6(net), IpAddr::V6(ip)) => {
            let net = u128::from(net);
            let ip = u128::from(ip);
            let mask = if prefix_len == 0 {
                0
            } else {
                u128::MAX << (128 - prefix_len)
            };
            (net & mask) == (ip & mask)
        }
        _ => false,
    }
}

#[derive(Clone, PartialEq, Message)]
struct GeoIpList {
    #[prost(message, repeated, tag = "1")]
    entry: Vec<GeoIpEntry>,
}

#[derive(Clone, PartialEq, Message)]
struct GeoIpEntry {
    #[prost(string, tag = "1")]
    country_code: String,
    #[prost(message, repeated, tag = "2")]
    cidr: Vec<GeoIpCidr>,
    #[prost(bool, tag = "3")]
    reverse_match: bool,
}

#[derive(Clone, PartialEq, Message)]
struct GeoIpCidr {
    #[prost(bytes = "vec", tag = "1")]
    ip: Vec<u8>,
    #[prost(uint32, tag = "2")]
    prefix: u32,
}

#[derive(Clone, PartialEq, Message)]
struct GeoSiteList {
    #[prost(message, repeated, tag = "1")]
    entry: Vec<GeoSiteEntry>,
}

#[derive(Clone, PartialEq, Message)]
struct GeoSiteEntry {
    #[prost(string, tag = "1")]
    country_code: String,
    #[prost(message, repeated, tag = "2")]
    domain: Vec<GeoSiteDomain>,
}

#[derive(Clone, PartialEq, Message)]
struct GeoSiteDomain {
    #[prost(enumeration = "GeoSiteDomainType", tag = "1")]
    r#type: i32,
    #[prost(string, tag = "2")]
    value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum GeoSiteDomainType {
    Plain = 0,
    Regex = 1,
    Domain = 2,
    Full = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geosite_dat_matches_domain_types() {
        let list = GeoSiteList {
            entry: vec![GeoSiteEntry {
                country_code: "test".to_string(),
                domain: vec![
                    GeoSiteDomain {
                        r#type: GeoSiteDomainType::Domain as i32,
                        value: "example.com".to_string(),
                    },
                    GeoSiteDomain {
                        r#type: GeoSiteDomainType::Full as i32,
                        value: "exact.test".to_string(),
                    },
                    GeoSiteDomain {
                        r#type: GeoSiteDomainType::Plain as i32,
                        value: "keyword".to_string(),
                    },
                    GeoSiteDomain {
                        r#type: GeoSiteDomainType::Regex as i32,
                        value: r"^re\d+\.test$".to_string(),
                    },
                ],
            }],
        };
        let data = GeoSiteData::from_geosite_dat_bytes(&list.encode_to_vec()).unwrap();

        assert!(data.matches("TEST", "www.example.com"));
        assert!(data.matches("test", "exact.test"));
        assert!(data.matches("test", "has-keyword.test"));
        assert!(data.matches("test", "re12.test"));
        assert!(!data.matches("test", "other.test"));
    }

    #[test]
    fn geoip_dat_matches_cidr_entries() {
        let list = GeoIpList {
            entry: vec![GeoIpEntry {
                country_code: "cn".to_string(),
                cidr: vec![GeoIpCidr {
                    ip: vec![203, 0, 113, 0],
                    prefix: 24,
                }],
                reverse_match: false,
            }],
        };
        let data = GeoIpData::from_geoip_dat_bytes(&list.encode_to_vec()).unwrap();

        assert!(data.matches("CN", "203.0.113.10".parse().unwrap()));
        assert!(!data.matches("CN", "198.51.100.10".parse().unwrap()));
    }
}
