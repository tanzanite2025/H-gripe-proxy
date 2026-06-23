use super::{RustDnsFallbackFilterGeoipDecisionEvidence, yaml::FallbackFilterGeoipConfig};
use smartstring::alias::String;
use std::net::{IpAddr, Ipv4Addr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GeoipCidr {
    code: &'static str,
    label: &'static str,
    network: u32,
    prefix: u8,
}

const GEOIP_CANARY_CIDRS: &[GeoipCidr] = &[
    GeoipCidr {
        code: "CN",
        label: "223.5.5.0/24",
        network: ipv4_network(223, 5, 5, 0),
        prefix: 24,
    },
    GeoipCidr {
        code: "CN",
        label: "119.29.29.0/24",
        network: ipv4_network(119, 29, 29, 0),
        prefix: 24,
    },
    GeoipCidr {
        code: "CN",
        label: "114.114.114.0/24",
        network: ipv4_network(114, 114, 114, 0),
        prefix: 24,
    },
    GeoipCidr {
        code: "US",
        label: "8.8.8.0/24",
        network: ipv4_network(8, 8, 8, 0),
        prefix: 24,
    },
];

pub(super) fn evaluate_geoip_filter(
    config: &FallbackFilterGeoipConfig,
    domain: &str,
    candidate_ip: IpAddr,
) -> RustDnsFallbackFilterGeoipDecisionEvidence {
    let matched_cidr = match candidate_ip {
        IpAddr::V4(ip) => GEOIP_CANARY_CIDRS
            .iter()
            .find(|cidr| cidr.code == config.geoip_code && cidr.matches(ip)),
        IpAddr::V6(_) => None,
    };
    let matched_country = matched_cidr.is_some();
    let fallback_required = config.geoip_enabled && !matched_country;

    RustDnsFallbackFilterGeoipDecisionEvidence {
        domain: domain.trim().trim_end_matches('.').to_ascii_lowercase().into(),
        candidate_ip: candidate_ip.to_string().into(),
        geoip_enabled: config.geoip_enabled,
        geoip_code: config.geoip_code.into(),
        matched_country,
        matched_cidr: matched_cidr.map(|cidr| String::from(cidr.label)),
        fallback_required,
        evaluated_cidr_count: GEOIP_CANARY_CIDRS
            .iter()
            .filter(|cidr| cidr.code == config.geoip_code)
            .count(),
    }
}

impl GeoipCidr {
    fn matches(self, ip: Ipv4Addr) -> bool {
        let mask = u32::MAX << (32 - u32::from(self.prefix));
        u32::from(ip) & mask == self.network & mask
    }
}

const fn ipv4_network(a: u8, b: u8, c: u8, d: u8) -> u32 {
    u32::from_be_bytes([a, b, c, d])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_cn_canary_match_without_fallback() {
        let config = FallbackFilterGeoipConfig {
            geoip_enabled: true,
            geoip_code: "CN",
        };

        let evidence = evaluate_geoip_filter(&config, "example.com", IpAddr::V4(Ipv4Addr::new(223, 5, 5, 5)));

        assert!(evidence.matched_country);
        assert!(!evidence.fallback_required);
    }

    #[test]
    fn marks_mismatch_as_fallback_required() {
        let config = FallbackFilterGeoipConfig {
            geoip_enabled: true,
            geoip_code: "CN",
        };

        let evidence = evaluate_geoip_filter(&config, "example.com", IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)));

        assert!(!evidence.matched_country);
        assert!(evidence.fallback_required);
    }
}
