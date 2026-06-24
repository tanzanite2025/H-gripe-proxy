//! Schema contract tests for the proxy deserialization layer.
//!
//! These assert the "compatible with all" guarantee: every protocol the
//! frontend can emit parses, XHTTP is typed, unknown protocols/fields degrade
//! gracefully instead of crashing, and routability is reported separately from
//! parseability.

use learn_gripe::{Network, ProtocolSupport, ProxyEntry, ProxyType};

fn parse(yaml: &str) -> ProxyEntry {
    serde_yaml_ng::from_str(yaml).expect("proxy entry should deserialize")
}

/// Every `type` string in the frontend `IProxyConfig` union must parse to a
/// concrete (non-`Unknown`) variant.
#[test]
fn all_frontend_protocol_types_parse() {
    let types = [
        ("ss", ProxyType::Shadowsocks),
        ("ssr", ProxyType::ShadowsocksR),
        ("direct", ProxyType::Direct),
        ("dns", ProxyType::Dns),
        ("snell", ProxyType::Snell),
        ("http", ProxyType::Http),
        ("trojan", ProxyType::Trojan),
        ("anytls", ProxyType::AnyTls),
        ("hysteria", ProxyType::Hysteria),
        ("hysteria2", ProxyType::Hysteria2),
        ("tuic", ProxyType::Tuic),
        ("wireguard", ProxyType::WireGuard),
        ("ssh", ProxyType::Ssh),
        ("socks5", ProxyType::Socks5),
        ("masque", ProxyType::Masque),
        ("gost-relay", ProxyType::GostRelay),
        ("trusttunnel", ProxyType::TrustTunnel),
        ("openvpn", ProxyType::OpenVpn),
        ("tailscale", ProxyType::Tailscale),
        ("reject", ProxyType::Reject),
        ("vmess", ProxyType::Vmess),
        ("vless", ProxyType::Vless),
        ("mieru", ProxyType::Mieru),
        ("sudoku", ProxyType::Sudoku),
    ];

    for (name, expected) in types {
        let entry = parse(&format!("name: probe\ntype: {name}\nserver: example.com\nport: 443\n"));
        assert_eq!(entry.kind, expected, "type `{name}` mapped to wrong variant");
        assert_ne!(entry.kind, ProxyType::Unknown, "type `{name}` fell through to Unknown");
    }
}

/// An unknown / future protocol must degrade to `Unknown` rather than error.
#[test]
fn unknown_protocol_type_is_tolerated() {
    let entry = parse("name: future\ntype: some-new-protocol-2027\nserver: x\nport: 1\n");
    assert_eq!(entry.kind, ProxyType::Unknown);
    assert_eq!(entry.support(), ProtocolSupport::Unsupported);
}

/// Unknown fields anywhere in the entry must be ignored, not rejected.
#[test]
fn unknown_fields_are_ignored() {
    let entry = parse(
        "name: lenient\ntype: ss\nserver: a\nport: 8388\ncipher: aes-128-gcm\n\
         password: pw\nsome-future-field: 123\nnested-future:\n  a: 1\n",
    );
    assert_eq!(entry.kind, ProxyType::Shadowsocks);
    assert_eq!(entry.options.cipher.as_deref(), Some("aes-128-gcm"));
}

/// XHTTP must be a first-class typed transport (network + xhttp-opts).
#[test]
fn xhttp_transport_is_typed() {
    let entry = parse(
        "name: x\ntype: vless\nserver: a\nport: 443\nuuid: u\ntls: true\nnetwork: xhttp\n\
         xhttp-opts:\n  path: /down\n  host: cdn.example.com\n  mode: packet-up\n\
         \n  no-grpc-header: true\n",
    );
    assert_eq!(entry.kind, ProxyType::Vless);
    assert_eq!(entry.options.network, Some(Network::Xhttp));
    let xhttp = entry.options.xhttp_opts.expect("xhttp-opts should be present");
    assert_eq!(xhttp.path.as_deref(), Some("/down"));
    assert_eq!(xhttp.host.as_deref(), Some("cdn.example.com"));
    assert_eq!(xhttp.mode.as_deref(), Some("packet-up"));
    assert_eq!(xhttp.no_grpc_header, Some(true));
}

/// All transport variants parse, including xhttp.
#[test]
fn all_transports_parse() {
    for net in ["tcp", "ws", "http", "h2", "grpc", "xhttp"] {
        let entry = parse(&format!(
            "name: t\ntype: vmess\nserver: a\nport: 443\nuuid: u\nnetwork: {net}\n"
        ));
        assert!(entry.options.network.is_some(), "network `{net}` failed to parse");
    }
}

/// Routability is reported independently from parseability.
#[test]
fn support_classification() {
    assert_eq!(parse("name: d\ntype: direct\n").support(), ProtocolSupport::Implemented);
    assert_eq!(parse("name: r\ntype: reject\n").support(), ProtocolSupport::Implemented);
    assert_eq!(
        parse("name: s\ntype: socks5\nserver: a\nport: 1\n").support(),
        ProtocolSupport::Implemented
    );
    // Parsed and typed, but no outbound data plane yet.
    assert_eq!(
        parse("name: v\ntype: vless\nserver: a\nport: 443\nuuid: u\n").support(),
        ProtocolSupport::Unsupported
    );
}

/// A full clash `proxies:` array of mixed protocols parses as a batch.
#[test]
fn mixed_proxies_array_parses() {
    let yaml = "\
- { name: a, type: ss, server: s, port: 8388, cipher: aes-128-gcm, password: p }
- { name: b, type: trojan, server: s, port: 443, password: p, sni: e.com }
- { name: c, type: vmess, server: s, port: 443, uuid: u, network: ws, ws-opts: { path: /v } }
- { name: d, type: vless, server: s, port: 443, uuid: u, network: xhttp, xhttp-opts: { mode: stream-up } }
- { name: e, type: hysteria2, server: s, port: 443, password: p }
- { name: f, type: exotic-future, server: s, port: 1 }
";
    let proxies: Vec<ProxyEntry> = serde_yaml_ng::from_str(yaml).expect("array should parse");
    assert_eq!(proxies.len(), 6);
    assert_eq!(proxies[5].kind, ProxyType::Unknown);
}
