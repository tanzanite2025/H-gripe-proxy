use super::provider::RuleProviderBehavior;
use super::*;
use crate::core::rule_geodata::{AsnData, GeoIpData, GeoSiteData, GeoSiteDomainType, RuleGeoData};
use std::{collections::HashMap, fs, path::PathBuf};

fn meta_domain(host: &str) -> ConnectionMeta {
    ConnectionMeta {
        host: host.to_owned(),
        ..Default::default()
    }
}

fn meta_ip(dst: &str, port: u16) -> ConnectionMeta {
    ConnectionMeta {
        dst_ip: Some(dst.parse().unwrap()),
        dst_port: port,
        ..Default::default()
    }
}

fn meta_process_name(process_name: &str) -> ConnectionMeta {
    ConnectionMeta {
        process_name: process_name.to_owned(),
        ..Default::default()
    }
}

fn meta_process_path(process_path: &str) -> ConnectionMeta {
    ConnectionMeta {
        process_path: process_path.to_owned(),
        ..Default::default()
    }
}

fn meta_uid(uid: u32) -> ConnectionMeta {
    ConnectionMeta {
        uid,
        ..Default::default()
    }
}

fn meta_dscp(dscp: u8) -> ConnectionMeta {
    ConnectionMeta {
        dscp,
        ..Default::default()
    }
}

fn meta_in_type(in_type: &str) -> ConnectionMeta {
    ConnectionMeta {
        in_type: in_type.to_owned(),
        ..Default::default()
    }
}

fn meta_in_user(in_user: &str) -> ConnectionMeta {
    ConnectionMeta {
        in_user: in_user.to_owned(),
        ..Default::default()
    }
}

fn meta_in_name(in_name: &str) -> ConnectionMeta {
    ConnectionMeta {
        in_name: in_name.to_owned(),
        ..Default::default()
    }
}

fn file_provider(path: PathBuf, behavior: RuleProviderBehavior) -> RuleProviderConfig {
    RuleProviderConfig {
        provider_type: "file".to_string(),
        behavior,
        path: Some(path),
        payload: Vec::new(),
        format: None,
    }
}

#[test]
fn domain_exact_match() {
    let engine = RuleEngine::from_rules(&["DOMAIN,google.com,Proxy", "MATCH,DIRECT"]).unwrap();
    let r = engine.match_connection(&meta_domain("google.com"));
    assert!(r.matched);
    assert_eq!(r.target.as_deref(), Some("Proxy"));
    assert_eq!(r.rule_type.as_deref(), Some("DOMAIN"));
}

#[test]
fn domain_suffix_match() {
    let engine = RuleEngine::from_rules(&["DOMAIN-SUFFIX,google.com,Proxy", "MATCH,DIRECT"]).unwrap();
    assert!(engine.match_connection(&meta_domain("www.google.com")).matched);
    assert!(engine.match_connection(&meta_domain("google.com")).matched);
    let r = engine.match_connection(&meta_domain("notgoogle.com"));
    assert_eq!(r.target.as_deref(), Some("DIRECT"));
}

#[test]
fn domain_keyword_match() {
    let engine = RuleEngine::from_rules(&["DOMAIN-KEYWORD,goog,Proxy", "MATCH,DIRECT"]).unwrap();
    assert!(engine.match_connection(&meta_domain("www.google.com")).matched);
    let r = engine.match_connection(&meta_domain("www.google.com"));
    assert_eq!(r.target.as_deref(), Some("Proxy"));
}

#[test]
fn domain_regex_match() {
    let engine = RuleEngine::from_rules(&["DOMAIN-REGEX,^(www\\.)?google\\.com$,Proxy", "MATCH,DIRECT"]).unwrap();
    assert!(engine.match_connection(&meta_domain("google.com")).matched);
    assert!(engine.match_connection(&meta_domain("www.google.com")).matched);
    let r = engine.match_connection(&meta_domain("mail.google.com"));
    assert_eq!(r.target.as_deref(), Some("DIRECT"));
}

#[test]
fn ip_cidr_match() {
    let engine = RuleEngine::from_rules(&["IP-CIDR,10.0.0.0/8,Direct", "MATCH,Proxy"]).unwrap();
    let r = engine.match_connection(&meta_ip("10.1.2.3", 80));
    assert_eq!(r.target.as_deref(), Some("Direct"));

    let r = engine.match_connection(&meta_ip("192.168.1.1", 80));
    assert_eq!(r.target.as_deref(), Some("Proxy"));
}

#[test]
fn port_range_match() {
    let engine = RuleEngine::from_rules(&["DST-PORT,80/443/8000-9000,Web", "MATCH,DIRECT"]).unwrap();
    assert_eq!(
        engine.match_connection(&meta_ip("1.2.3.4", 443)).target.as_deref(),
        Some("Web")
    );
    assert_eq!(
        engine.match_connection(&meta_ip("1.2.3.4", 8500)).target.as_deref(),
        Some("Web")
    );
    assert_eq!(
        engine.match_connection(&meta_ip("1.2.3.4", 22)).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn network_match() {
    let engine = RuleEngine::from_rules(&["NETWORK,udp,UdpProxy", "MATCH,DIRECT"]).unwrap();
    let mut m = meta_domain("example.com");
    m.network = NetworkType::Udp;
    assert_eq!(engine.match_connection(&m).target.as_deref(), Some("UdpProxy"));
}

#[test]
fn logical_and_matches_all_child_rules() {
    let engine = RuleEngine::from_rules(&[
        "AND,((DOMAIN,example.com),(NETWORK,TCP),(DST-PORT,443)),Proxy",
        "MATCH,DIRECT",
    ])
    .unwrap();
    let mut meta = meta_domain("example.com");
    meta.network = NetworkType::Tcp;
    meta.dst_port = 443;

    assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("Proxy"));

    meta.dst_port = 80;
    assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("DIRECT"));
}

#[test]
fn logical_or_and_not_match_mihomo_payload_shape() {
    let engine =
        RuleEngine::from_rules(&["OR,((DOMAIN,example.com),(NOT,((NETWORK,UDP)))),Proxy", "MATCH,DIRECT"]).unwrap();
    let mut meta = meta_domain("other.example");
    meta.network = NetworkType::Tcp;

    assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("Proxy"));

    meta.network = NetworkType::Udp;
    assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("DIRECT"));
}

#[test]
fn sub_rule_routes_to_named_rule_list() {
    let sub_rules = SubRuleData::from_sub_rules(HashMap::from([(
        "sub-rule-name1".to_string(),
        vec!["DOMAIN,example.com,Proxy".to_string(), "MATCH,DIRECT".to_string()],
    )]))
    .unwrap();
    let engine = RuleEngine::from_rules_with_default_geo_data_rule_sets_and_sub_rules(
        &[
            "SUB-RULE,(OR,((NETWORK,TCP),(NETWORK,UDP))),sub-rule-name1",
            "MATCH,FALLBACK",
        ],
        RuleSetData::empty(),
        sub_rules,
    )
    .unwrap();
    let mut meta = meta_domain("example.com");
    meta.network = NetworkType::Tcp;

    assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("Proxy"));
}

#[test]
fn sub_rule_rejects_missing_or_circular_references() {
    assert!(
        SubRuleData::from_sub_rules(HashMap::from([(
            "first".to_string(),
            vec!["SUB-RULE,(DOMAIN,example.com),missing".to_string()],
        )]))
        .is_err()
    );
    assert!(
        SubRuleData::from_sub_rules(HashMap::from([
            (
                "first".to_string(),
                vec!["SUB-RULE,(DOMAIN,example.com),second".to_string()],
            ),
            (
                "second".to_string(),
                vec!["SUB-RULE,(DOMAIN,example.org),first".to_string()],
            ),
        ]))
        .is_err()
    );
}

#[test]
fn process_name_matches_case_insensitively() {
    let engine = RuleEngine::from_rules(&["PROCESS-NAME,Telegram.exe,Proxy", "MATCH,DIRECT"]).unwrap();
    let result = engine.match_connection(&meta_process_name("telegram.EXE"));

    assert_eq!(result.target.as_deref(), Some("Proxy"));
    assert_eq!(result.rule_type.as_deref(), Some("PROCESS-NAME"));
}

#[test]
fn process_name_without_metadata_falls_through() {
    let engine = RuleEngine::from_rules(&["PROCESS-NAME,Telegram.exe,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn process_path_matches_case_insensitively() {
    let engine = RuleEngine::from_rules(&[
        "PROCESS-PATH,C:\\Program Files\\Telegram\\Telegram.exe,Proxy",
        "MATCH,DIRECT",
    ])
    .unwrap();
    let result = engine.match_connection(&meta_process_path("c:\\program files\\telegram\\telegram.EXE"));

    assert_eq!(result.target.as_deref(), Some("Proxy"));
    assert_eq!(result.rule_type.as_deref(), Some("PROCESS-PATH"));
}

#[test]
fn process_path_without_metadata_falls_through() {
    let engine = RuleEngine::from_rules(&[
        "PROCESS-PATH,C:\\Program Files\\Telegram\\Telegram.exe,Proxy",
        "MATCH,DIRECT",
    ])
    .unwrap();

    assert_eq!(
        engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn process_name_regex_matches_case_insensitively() {
    let engine =
        RuleEngine::from_rules(&["PROCESS-NAME-REGEX,^telegram(-desktop)?\\.exe$,Proxy", "MATCH,DIRECT"]).unwrap();
    let result = engine.match_connection(&meta_process_name("Telegram-Desktop.EXE"));

    assert_eq!(result.target.as_deref(), Some("Proxy"));
    assert_eq!(result.rule_type.as_deref(), Some("PROCESS-NAME-REGEX"));
}

#[test]
fn process_name_regex_without_metadata_falls_through() {
    let engine = RuleEngine::from_rules(&["PROCESS-NAME-REGEX,^telegram\\.exe$,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn process_path_regex_matches_case_insensitively() {
    let engine = RuleEngine::from_rules(&[
        "PROCESS-PATH-REGEX,^c:\\\\program files\\\\telegram\\\\telegram\\.exe$,Proxy",
        "MATCH,DIRECT",
    ])
    .unwrap();
    let result = engine.match_connection(&meta_process_path("C:\\Program Files\\Telegram\\Telegram.EXE"));

    assert_eq!(result.target.as_deref(), Some("Proxy"));
    assert_eq!(result.rule_type.as_deref(), Some("PROCESS-PATH-REGEX"));
}

#[test]
fn process_path_regex_without_metadata_falls_through() {
    let engine = RuleEngine::from_rules(&["PROCESS-PATH-REGEX,telegram\\.exe$,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn process_name_wildcard_matches_case_insensitively() {
    let engine = RuleEngine::from_rules(&["PROCESS-NAME-WILDCARD,*telegram?.exe,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine
            .match_connection(&meta_process_name("ORG.Telegram1.EXE"))
            .target
            .as_deref(),
        Some("Proxy")
    );
    assert_eq!(
        engine
            .match_connection(&meta_process_name("firefox.exe"))
            .target
            .as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn process_path_wildcard_matches_case_insensitively() {
    let engine =
        RuleEngine::from_rules(&["PROCESS-PATH-WILDCARD,*\\telegram\\telegram.exe,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine
            .match_connection(&meta_process_path("C:\\Users\\Alice\\Telegram\\Telegram.EXE"))
            .target
            .as_deref(),
        Some("Proxy")
    );
    assert_eq!(
        engine
            .match_connection(&meta_process_path("C:\\Program Files\\Firefox\\firefox.exe"))
            .target
            .as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn process_wildcard_without_metadata_falls_through() {
    let engine = RuleEngine::from_rules(&["PROCESS-NAME-WILDCARD,*telegram*,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn uid_matches_single_value_and_ranges() {
    let engine = RuleEngine::from_rules(&["UID,1000/2000-2002,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&meta_uid(1000)).target.as_deref(),
        Some("Proxy")
    );
    assert_eq!(
        engine.match_connection(&meta_uid(2001)).target.as_deref(),
        Some("Proxy")
    );
    assert_eq!(
        engine.match_connection(&meta_uid(3000)).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn uid_without_metadata_falls_through() {
    let engine = RuleEngine::from_rules(&["UID,1000,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn dscp_matches_single_value_and_ranges() {
    let engine = RuleEngine::from_rules(&["DSCP,10/46-48,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(engine.match_connection(&meta_dscp(10)).target.as_deref(), Some("Proxy"));
    assert_eq!(engine.match_connection(&meta_dscp(47)).target.as_deref(), Some("Proxy"));
    assert_eq!(
        engine.match_connection(&meta_dscp(20)).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn dscp_wildcard_matches_default_metadata() {
    let engine = RuleEngine::from_rules(&["DSCP,*,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
        Some("Proxy")
    );
}

#[test]
fn in_type_matches_case_insensitively() {
    let engine = RuleEngine::from_rules(&["IN-TYPE,HTTP/TUN,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&meta_in_type("http")).target.as_deref(),
        Some("Proxy")
    );
    assert_eq!(
        engine.match_connection(&meta_in_type("Tun")).target.as_deref(),
        Some("Proxy")
    );
    assert_eq!(
        engine.match_connection(&meta_in_type("HTTPS")).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn in_type_socks_expands_to_socks4_and_socks5() {
    let engine = RuleEngine::from_rules(&["IN-TYPE,SOCKS,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&meta_in_type("socks4")).target.as_deref(),
        Some("Proxy")
    );
    assert_eq!(
        engine.match_connection(&meta_in_type("Socks5")).target.as_deref(),
        Some("Proxy")
    );
}

#[test]
fn in_type_without_metadata_falls_through() {
    let engine = RuleEngine::from_rules(&["IN-TYPE,HTTP,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn in_user_matches_exactly() {
    let engine = RuleEngine::from_rules(&["IN-USER,alice/bob,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&meta_in_user("alice")).target.as_deref(),
        Some("Proxy")
    );
    assert_eq!(
        engine.match_connection(&meta_in_user("bob")).target.as_deref(),
        Some("Proxy")
    );
    assert_eq!(
        engine.match_connection(&meta_in_user("Alice")).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn in_user_without_metadata_falls_through() {
    let engine = RuleEngine::from_rules(&["IN-USER,alice,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn in_name_matches_exactly() {
    let engine = RuleEngine::from_rules(&["IN-NAME,home/work,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&meta_in_name("home")).target.as_deref(),
        Some("Proxy")
    );
    assert_eq!(
        engine.match_connection(&meta_in_name("work")).target.as_deref(),
        Some("Proxy")
    );
    assert_eq!(
        engine.match_connection(&meta_in_name("Home")).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn in_name_without_metadata_falls_through() {
    let engine = RuleEngine::from_rules(&["IN-NAME,home,Proxy", "MATCH,DIRECT"]).unwrap();

    assert_eq!(
        engine.match_connection(&ConnectionMeta::default()).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn wildcard_match_cases() {
    assert!(wildcard_match("*.google.com", "www.google.com"));
    assert!(wildcard_match("*.google.com", "mail.google.com"));
    assert!(!wildcard_match("*.google.com", "google.com"));
    assert!(wildcard_match("google.*", "google.com"));
    assert!(wildcard_match("g?ogle.com", "google.com"));
}

#[test]
fn validate_rule_catches_errors() {
    assert!(validate_rule("DOMAIN,google.com,Proxy").valid);
    assert!(validate_rule("IP-CIDR,10.0.0.0/8,Direct").valid);
    assert!(validate_rule("PROCESS-NAME,Telegram.exe,Proxy").valid);
    assert!(validate_rule("PROCESS-PATH,C:\\Program Files\\Telegram\\Telegram.exe,Proxy").valid);
    assert!(validate_rule("PROCESS-NAME-REGEX,^telegram(-desktop)?\\.exe$,Proxy").valid);
    assert!(validate_rule("PROCESS-PATH-REGEX,telegram\\.exe$,Proxy").valid);
    assert!(validate_rule("PROCESS-NAME-WILDCARD,*telegram*,Proxy").valid);
    assert!(validate_rule("PROCESS-PATH-WILDCARD,*\\telegram\\telegram.exe,Proxy").valid);
    assert!(validate_rule("UID,1000/2000-2002,Proxy").valid);
    assert!(validate_rule("DSCP,10/46-48,Proxy").valid);
    assert!(validate_rule("DSCP,*,Proxy").valid);
    assert!(validate_rule("IN-TYPE,HTTP/SOCKS/TUN,Proxy").valid);
    assert!(validate_rule("IN-USER,alice/bob,Proxy").valid);
    assert!(validate_rule("IN-NAME,home/work,Proxy").valid);
    assert!(validate_rule("MATCH,DIRECT").valid);

    assert!(!validate_rule("DOMAIN").valid);
    assert!(!validate_rule("PROCESS-NAME-REGEX,[,Proxy").valid);
    assert!(!validate_rule("PROCESS-PATH-REGEX,[,Proxy").valid);
    assert!(!validate_rule("UID,*,Proxy").valid);
    assert!(!validate_rule("UID,not-a-uid,Proxy").valid);
    assert!(!validate_rule("DSCP,64,Proxy").valid);
    assert!(!validate_rule("DSCP,not-a-dscp,Proxy").valid);
    assert!(!validate_rule("IN-TYPE,,Proxy").valid);
    assert!(!validate_rule("IN-TYPE,UNKNOWN,Proxy").valid);
    assert!(!validate_rule("IN-USER,,Proxy").valid);
    assert!(!validate_rule("IN-NAME,,Proxy").valid);
    assert!(!validate_rule("IP-CIDR,not-a-cidr,Direct").valid);
    assert!(!validate_rule("IP-SUFFIX,1.2.3.4/40,Direct").valid);
    assert!(!validate_rule("IP-ASN,not-a-number,Direct").valid);
    assert!(!validate_rule("DST-PORT,notaport,Direct").valid);
    assert!(!validate_rule("UNKNOWN-TYPE,foo,bar").valid);
}

#[test]
fn geoip_falls_through_without_geodata() {
    let v = validate_rule("GEOIP,CN,DIRECT,no-resolve");
    assert!(v.valid);
    let engine = RuleEngine::from_rules(&["GEOIP,CN,DIRECT,no-resolve", "MATCH,Proxy"]).unwrap();
    let r = engine.match_connection(&meta_ip("1.2.3.4", 80));
    assert_eq!(r.target.as_deref(), Some("Proxy"));
}

#[test]
fn geoip_matches_with_geodata() {
    let geoip = GeoIpData::from_cidr_map(HashMap::from([(
        "cn".to_string(),
        vec![("203.0.113.0".parse().unwrap(), 24)],
    )]));
    let geo_data = RuleGeoData::from_parts(Some(geoip), None, None);
    let engine =
        RuleEngine::from_rules_with_geo_data(&["GEOIP,CN,DIRECT,no-resolve", "MATCH,Proxy"], geo_data).unwrap();

    assert_eq!(
        engine.match_connection(&meta_ip("203.0.113.10", 80)).target.as_deref(),
        Some("DIRECT")
    );
    assert_eq!(
        engine.match_connection(&meta_ip("198.51.100.10", 80)).target.as_deref(),
        Some("Proxy")
    );
}

#[test]
fn geosite_matches_with_geodata() {
    let geosite = GeoSiteData::from_site_map(HashMap::from([(
        "cn".to_string(),
        vec![(GeoSiteDomainType::Domain, "example.cn".to_string())],
    )]))
    .unwrap();
    let geo_data = RuleGeoData::from_parts(None, Some(geosite), None);
    let engine = RuleEngine::from_rules_with_geo_data(&["GEOSITE,CN,DIRECT", "MATCH,Proxy"], geo_data).unwrap();

    assert_eq!(
        engine
            .match_connection(&meta_domain("www.example.cn"))
            .target
            .as_deref(),
        Some("DIRECT")
    );
    assert_eq!(
        engine
            .match_connection(&meta_domain("www.example.com"))
            .target
            .as_deref(),
        Some("Proxy")
    );
}

#[test]
fn geoip_lan_matches_without_external_data() {
    let engine = RuleEngine::from_rules(&["GEOIP,LAN,DIRECT", "MATCH,Proxy"]).unwrap();
    assert_eq!(
        engine.match_connection(&meta_ip("192.168.1.1", 80)).target.as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn ip_asn_falls_through_without_geodata() {
    let v = validate_rule("IP-ASN,13335,DIRECT");
    assert!(v.valid);
    let engine = RuleEngine::from_rules(&["IP-ASN,13335,DIRECT", "MATCH,Proxy"]).unwrap();
    assert_eq!(
        engine.match_connection(&meta_ip("1.1.1.1", 443)).target.as_deref(),
        Some("Proxy")
    );
}

#[test]
fn ip_asn_matches_destination_with_geodata() {
    let asn_data = AsnData::from_asn_map(HashMap::from([("1.1.1.1".parse().unwrap(), 13335)]));
    let geo_data = RuleGeoData::from_parts(None, None, Some(asn_data));
    let engine = RuleEngine::from_rules_with_geo_data(&["IP-ASN,13335,DIRECT", "MATCH,Proxy"], geo_data).unwrap();

    assert_eq!(
        engine.match_connection(&meta_ip("1.1.1.1", 443)).target.as_deref(),
        Some("DIRECT")
    );
    assert_eq!(
        engine.match_connection(&meta_ip("8.8.8.8", 443)).target.as_deref(),
        Some("Proxy")
    );
}

#[test]
fn src_ip_asn_matches_source_with_geodata() {
    let asn_data = AsnData::from_asn_map(HashMap::from([("8.8.8.8".parse().unwrap(), 15169)]));
    let geo_data = RuleGeoData::from_parts(None, None, Some(asn_data));
    let engine = RuleEngine::from_rules_with_geo_data(&["SRC-IP-ASN,15169,Proxy", "MATCH,DIRECT"], geo_data).unwrap();
    let mut meta = meta_ip("1.1.1.1", 443);
    meta.src_ip = Some("8.8.8.8".parse().unwrap());

    assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("Proxy"));
}

#[test]
fn ip_asn_accepts_ipv6_query_path() {
    let asn_data = AsnData::from_asn_map(HashMap::from([("2606:4700:4700::1111".parse().unwrap(), 13335)]));
    let geo_data = RuleGeoData::from_parts(None, None, Some(asn_data));
    let engine = RuleEngine::from_rules_with_geo_data(&["IP-ASN,13335,DIRECT", "MATCH,Proxy"], geo_data).unwrap();
    let mut meta = ConnectionMeta::default();
    meta.dst_ip = Some("2606:4700:4700::1111".parse().unwrap());

    assert_eq!(engine.match_connection(&meta).target.as_deref(), Some("DIRECT"));
}

#[test]
fn rule_set_yaml_domain_provider_matches_outer_target() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private.yaml");
    fs::write(&path, "payload:\n  - DOMAIN-SUFFIX,example.com\n").unwrap();
    let rule_sets = RuleSetData::from_rule_providers(HashMap::from([(
        "private".to_string(),
        file_provider(path, RuleProviderBehavior::Classical),
    )]))
    .unwrap();
    let engine = RuleEngine::from_rules_with_rule_sets(&["RULE-SET,private,DIRECT", "MATCH,Proxy"], rule_sets).unwrap();

    assert_eq!(
        engine
            .match_connection(&meta_domain("www.example.com"))
            .target
            .as_deref(),
        Some("DIRECT")
    );
    assert_eq!(
        engine
            .match_connection(&meta_domain("www.example.net"))
            .target
            .as_deref(),
        Some("Proxy")
    );
}

#[test]
fn rule_set_yaml_ipcidr_provider_matches_ip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private-ip.yaml");
    fs::write(&path, "payload:\n  - 10.0.0.0/8\n").unwrap();
    let rule_sets = RuleSetData::from_rule_providers(HashMap::from([(
        "private-ip".to_string(),
        file_provider(path, RuleProviderBehavior::Ipcidr),
    )]))
    .unwrap();
    let engine =
        RuleEngine::from_rules_with_rule_sets(&["RULE-SET,private-ip,DIRECT", "MATCH,Proxy"], rule_sets).unwrap();

    assert_eq!(
        engine.match_connection(&meta_ip("10.1.2.3", 443)).target.as_deref(),
        Some("DIRECT")
    );
    assert_eq!(
        engine
            .match_connection(&meta_ip("198.51.100.10", 443))
            .target
            .as_deref(),
        Some("Proxy")
    );
}

#[test]
fn rule_set_text_domain_provider_matches_domain_suffix() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("domain.txt");
    fs::write(&path, "example.org\n# ignored\n\n").unwrap();
    let rule_sets = RuleSetData::from_rule_providers(HashMap::from([(
        "domain".to_string(),
        file_provider(path, RuleProviderBehavior::Domain),
    )]))
    .unwrap();
    let engine = RuleEngine::from_rules_with_rule_sets(&["RULE-SET,domain,Proxy", "MATCH,DIRECT"], rule_sets).unwrap();

    assert_eq!(
        engine
            .match_connection(&meta_domain("api.example.org"))
            .target
            .as_deref(),
        Some("Proxy")
    );
}

#[test]
fn rule_set_outer_target_overrides_provider_rule_target() {
    let provider = RuleProviderConfig {
        provider_type: "inline".to_string(),
        behavior: RuleProviderBehavior::Classical,
        path: None,
        payload: vec!["DOMAIN-SUFFIX,example.com,REJECT".to_string()],
        format: None,
    };
    let rule_sets = RuleSetData::from_rule_providers(HashMap::from([("reject".to_string(), provider)])).unwrap();
    let engine = RuleEngine::from_rules_with_rule_sets(&["RULE-SET,reject,DIRECT", "MATCH,Proxy"], rule_sets).unwrap();

    assert_eq!(
        engine
            .match_connection(&meta_domain("www.example.com"))
            .target
            .as_deref(),
        Some("DIRECT")
    );
}

#[test]
fn rule_set_missing_provider_falls_through() {
    let engine =
        RuleEngine::from_rules_with_rule_sets(&["RULE-SET,missing,DIRECT", "MATCH,Proxy"], RuleSetData::empty())
            .unwrap();

    assert_eq!(
        engine
            .match_connection(&meta_domain("www.example.com"))
            .target
            .as_deref(),
        Some("Proxy")
    );
}

#[test]
fn rule_set_nested_provider_is_rejected() {
    let provider = RuleProviderConfig {
        provider_type: "inline".to_string(),
        behavior: RuleProviderBehavior::Classical,
        path: None,
        payload: vec!["RULE-SET,other".to_string()],
        format: None,
    };

    assert!(RuleSetData::from_rule_providers(HashMap::from([("nested".to_string(), provider)])).is_err());
}

#[test]
fn explain_connection_records_match_trace() {
    let engine = RuleEngine::from_rules(&["DOMAIN,example.net,Proxy", "DOMAIN,example.com,DIRECT"]).unwrap();
    let result = engine.explain_connection(&meta_domain("example.com"));

    assert!(result.matched);
    assert_eq!(result.outcome, "matched");
    assert_eq!(result.rule_index, Some(1));
    assert_eq!(result.rule_type.as_deref(), Some("DOMAIN"));
    assert_eq!(result.target.as_deref(), Some("DIRECT"));
    assert_eq!(result.trace.len(), 2);
    assert!(!result.trace[0].matched);
    assert_eq!(result.trace[1].rule_raw, "DOMAIN,example.com,DIRECT");
    assert_eq!(result.trace[1].target.as_deref(), Some("DIRECT"));
}

#[test]
fn explain_connection_records_fallthrough_trace() {
    let engine = RuleEngine::from_rules(&["DOMAIN,example.net,Proxy"]).unwrap();
    let result = engine.explain_connection(&meta_domain("example.com"));

    assert!(!result.matched);
    assert_eq!(result.outcome, "fallthrough");
    assert_eq!(result.explanation, "no rules matched; fallthrough without target");
    assert_eq!(result.trace.len(), 1);
    assert_eq!(result.trace[0].rule_type, "DOMAIN");
    assert!(!result.trace[0].matched);
}

#[test]
fn explain_connection_shows_rule_set_inner_match() {
    let provider = RuleProviderConfig {
        provider_type: "inline".to_string(),
        behavior: RuleProviderBehavior::Classical,
        path: None,
        payload: vec!["DOMAIN-SUFFIX,example.com,REJECT".to_string()],
        format: None,
    };
    let rule_sets = RuleSetData::from_rule_providers(HashMap::from([("private".to_string(), provider)])).unwrap();
    let engine = RuleEngine::from_rules_with_rule_sets(&["RULE-SET,private,DIRECT", "MATCH,Proxy"], rule_sets).unwrap();
    let result = engine.explain_connection(&meta_domain("www.example.com"));
    let detail = result.trace[0].detail.as_ref().unwrap();

    assert_eq!(result.target.as_deref(), Some("DIRECT"));
    assert_eq!(detail.reference_type, "rule_set");
    assert_eq!(detail.name, "private");
    assert_eq!(
        detail.matched_rule_raw.as_deref(),
        Some("DOMAIN-SUFFIX,example.com,__RULE_SET_MATCH__")
    );
    assert_eq!(detail.matched_rule_type.as_deref(), Some("DOMAIN-SUFFIX"));
}

#[test]
fn explain_connection_shows_sub_rule_inner_match() {
    let sub_rules = SubRuleData::from_sub_rules(HashMap::from([(
        "domain-preview".to_string(),
        vec!["DOMAIN-SUFFIX,example.com,Proxy".to_string()],
    )]))
    .unwrap();
    let engine = RuleEngine::from_rules_with_geo_data_rule_sets_and_sub_rules(
        &["SUB-RULE,DOMAIN-SUFFIX,example.com,domain-preview", "MATCH,DIRECT"],
        RuleGeoData::empty(),
        RuleSetData::empty(),
        sub_rules,
    )
    .unwrap();
    let result = engine.explain_connection(&meta_domain("www.example.com"));
    let detail = result.trace[0].detail.as_ref().unwrap();

    assert_eq!(result.target.as_deref(), Some("Proxy"));
    assert_eq!(detail.reference_type, "sub_rule");
    assert_eq!(detail.name, "domain-preview");
    assert_eq!(detail.condition_matched, Some(true));
    assert_eq!(
        detail.matched_rule_raw.as_deref(),
        Some("DOMAIN-SUFFIX,example.com,Proxy")
    );
    assert_eq!(detail.matched_rule_type.as_deref(), Some("DOMAIN-SUFFIX"));
    assert_eq!(detail.matched_target.as_deref(), Some("Proxy"));
}

#[test]
fn cidr_v6() {
    let engine = RuleEngine::from_rules(&["IP-CIDR6,fd00::/8,Local", "MATCH,Proxy"]).unwrap();
    let mut m = ConnectionMeta::default();
    m.dst_ip = Some("fd12:3456::1".parse().unwrap());
    assert_eq!(engine.match_connection(&m).target.as_deref(), Some("Local"));
}

#[test]
fn domain_regex_payload_with_comma() {
    let engine = RuleEngine::from_rules(&["DOMAIN-REGEX,^(foo|bar).example\\.com$,Proxy", "MATCH,DIRECT"]).unwrap();
    assert!(engine.match_connection(&meta_domain("foo.example.com")).matched);
}
