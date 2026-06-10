use serde_yaml_ng::Value;

pub const DOMESTIC_PLAIN_NAMESERVERS: &[&str] = &["223.5.5.5"];
pub const DOMESTIC_DOH_NAMESERVERS: &[&str] = &["https://dns.alidns.com/dns-query"];
pub const DOMESTIC_DOT_NAMESERVERS: &[&str] = &["tls://dns.alidns.com:853"];

pub const FOREIGN_PLAIN_NAMESERVERS: &[&str] = &["1.1.1.1"];
pub const FOREIGN_DOH_NAMESERVERS: &[&str] = &["https://cloudflare-dns.com/dns-query"];
pub const FOREIGN_DOT_NAMESERVERS: &[&str] = &["tls://1.1.1.1:853"];

pub const ENCRYPTED_BOOTSTRAP_NAMESERVERS: &[&str] = &["https://1.1.1.1/dns-query"];

pub fn value_sequence(values: &[&str]) -> Vec<Value> {
    values
        .iter()
        .map(|value| Value::String((*value).into()))
        .collect()
}
