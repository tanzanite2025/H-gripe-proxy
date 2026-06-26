//! Rule-based outbound selection.
//!
//! A [`Router`] holds a set of named outbounds plus an ordered list of
//! [`Rule`]s. For each connection the rules are evaluated top-to-bottom and the
//! first matching rule decides which named outbound the connection takes; if no
//! rule matches, the router's `fallback` outbound is used (the idiomatic Clash
//! `MATCH` catch-all can be expressed either as a trailing `MATCH` rule or via
//! `fallback`).
//!
//! Two outbound names are always available without being declared:
//! `DIRECT` (connect straight to the target) and `REJECT` (refuse the
//! connection). This mirrors Clash's built-in policies.
//!
//! Scope: `DOMAIN`, `DOMAIN-SUFFIX`, `DOMAIN-KEYWORD`, `IP-CIDR` (v4 and v6),
//! `DST-PORT`, `SRC-PORT`, `NETWORK`, `PROCESS-NAME`, `PROCESS-PATH`, `MATCH`,
//! plus `GEOIP` / `GEOSITE` / `IP-ASN` / `SRC-IP-ASN` and `RULE-SET`. The geo matchers
//! carry a shared [`GeoLookup`] handle to a locally-maintained geo database
//! (mmdb / geosite `.dat`), `RULE-SET` carries a shared [`RuleSetLookup`]
//! handle to locally-loaded rule providers, and the process matchers carry a
//! shared [`ProcessLookup`] handle that maps a connection's source socket to
//! the owning local process; the kernel never fetches that data itself — the
//! embedder loads the local files / performs the OS lookup and supplies the
//! handle, keeping data sourcing out of the data plane.
//!
//! The module is split by concern: the external lookup abstractions live in
//! [`lookup`], the parsed value types in [`types`], and the [`RuleMatcher`]
//! predicate tree in [`matcher`]; this file holds the [`Router`] that
//! orchestrates them. All public items are re-exported here so the kernel's
//! public surface is unchanged.

pub mod delay;
pub mod lookup;
pub mod matcher;
pub mod types;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::net::SocketAddr;

use anyhow::{Result, bail};

use crate::address::TargetAddr;
use crate::config::OutboundMode;
use crate::conntrack::ConnNetwork;

pub use lookup::{GeoLookup, ProcessInfo, ProcessLookup, RuleSetLookup};
pub use matcher::{LogicalOp, RuleMatcher};
pub use types::{IpCidr, PortRange, UidRange};

/// Built-in outbound name that connects straight to the target.
pub const DIRECT: &str = "DIRECT";
/// Built-in outbound name that refuses the connection.
pub const REJECT: &str = "REJECT";

/// A rule: a predicate and the name of the outbound to use when it matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub matcher: RuleMatcher,
    pub outbound: String,
}

impl Rule {
    pub fn new(matcher: RuleMatcher, outbound: impl Into<String>) -> Self {
        Self {
            matcher,
            outbound: outbound.into(),
        }
    }
}

/// Rule-based outbound selector. Build with [`Router::new`], which validates
/// that every referenced outbound name resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Router {
    outbounds: HashMap<String, OutboundMode>,
    rules: Vec<Rule>,
    fallback: String,
}

impl Router {
    /// Build a router from named `outbounds`, ordered `rules`, and a `fallback`
    /// outbound name used when no rule matches. The built-in `DIRECT` and
    /// `REJECT` names are always resolvable and may be referenced without being
    /// present in `outbounds`. Returns an error if any rule target or the
    /// fallback names an outbound that does not resolve.
    pub fn new(
        outbounds: HashMap<String, OutboundMode>,
        rules: Vec<Rule>,
        fallback: impl Into<String>,
    ) -> Result<Self> {
        let fallback = fallback.into();
        let router = Self {
            outbounds,
            rules,
            fallback,
        };
        for rule in &router.rules {
            if router.lookup(&rule.outbound).is_none() {
                bail!("router: rule references unknown outbound {:?}", rule.outbound);
            }
        }
        if router.lookup(&router.fallback).is_none() {
            bail!("router: fallback references unknown outbound {:?}", router.fallback);
        }
        Ok(router)
    }

    /// Resolve an outbound name, honouring the built-in `DIRECT` / `REJECT`
    /// policies. Built-ins are shadowed if explicitly declared in `outbounds`.
    fn lookup<'a>(&'a self, name: &str) -> Option<&'a OutboundMode> {
        if let Some(mode) = self.outbounds.get(name) {
            return Some(mode);
        }
        match name {
            DIRECT => Some(&OutboundMode::Direct),
            REJECT => Some(&OutboundMode::Reject),
            _ => None,
        }
    }

    /// The distinct named outbounds this router can select. The built-in
    /// `DIRECT`/`REJECT` policies carry no server and are not included.
    pub fn outbound_modes(&self) -> impl Iterator<Item = &OutboundMode> {
        self.outbounds.values()
    }

    /// Select the outbound for `target` on a TCP connection: the first matching
    /// rule's outbound, or the fallback. The name is guaranteed to resolve
    /// (checked in [`Router::new`]).
    ///
    /// Convenience wrapper over [`select_network`](Router::select_network) that
    /// assumes [`ConnNetwork::Tcp`]; use `select_network` when the connection's
    /// protocol is known so that `NETWORK` rules evaluate correctly.
    pub fn select(&self, target: &TargetAddr) -> &OutboundMode {
        self.select_network(target, ConnNetwork::Tcp)
    }

    /// Select the outbound for `target` on a connection of the given `network`
    /// (transport protocol). Behaves like [`select`](Router::select) but lets
    /// `NETWORK` rules match the protocol. Convenience wrapper over
    /// [`select_conn`](Router::select_conn) with an unknown source.
    pub fn select_network(&self, target: &TargetAddr, network: ConnNetwork) -> &OutboundMode {
        self.select_conn(target, network, None)
    }

    /// Select the outbound for a connection to `target` over `network` from
    /// source `src`. Behaves like [`select_network`](Router::select_network)
    /// but also lets `SRC-PORT` rules match the source port. `src` is `None`
    /// when the embedder cannot supply the source.
    pub fn select_conn(&self, target: &TargetAddr, network: ConnNetwork, src: Option<SocketAddr>) -> &OutboundMode {
        self.select_detailed_conn(target, network, src).outbound
    }

    /// Like [`select`](Router::select) but also reports the chosen outbound's
    /// name and the rule that matched (if any), for connection bookkeeping.
    pub fn select_detailed<'a>(&'a self, target: &TargetAddr) -> Selection<'a> {
        self.select_detailed_network(target, ConnNetwork::Tcp)
    }

    /// Like [`select_detailed`](Router::select_detailed) but for a connection of
    /// the given `network`, so `NETWORK` rules evaluate against the protocol.
    /// Convenience wrapper over [`select_detailed_conn`](Router::select_detailed_conn)
    /// with an unknown source.
    pub fn select_detailed_network<'a>(&'a self, target: &TargetAddr, network: ConnNetwork) -> Selection<'a> {
        self.select_detailed_conn(target, network, None)
    }

    /// Like [`select_detailed_network`](Router::select_detailed_network) but
    /// also carries the connection's source `src`, so `SRC-PORT` rules evaluate
    /// against the source port. `src` is `None` when the embedder cannot supply
    /// the source.
    pub fn select_detailed_conn<'a>(
        &'a self,
        target: &TargetAddr,
        network: ConnNetwork,
        src: Option<SocketAddr>,
    ) -> Selection<'a> {
        let matched = self
            .rules
            .iter()
            .find(|rule| rule.matcher.matches_conn(target, network, src));
        let name = matched.map(|rule| rule.outbound.as_str()).unwrap_or(&self.fallback);
        Selection {
            outbound_name: name,
            outbound: self.lookup(name).unwrap_or(&OutboundMode::Reject),
            rule: matched,
        }
    }
}

/// The outcome of resolving a [`Router`] for one target: which outbound was
/// chosen, its name, and the rule that selected it (`None` when the fallback
/// was used).
#[derive(Debug)]
pub struct Selection<'a> {
    pub outbound_name: &'a str,
    pub outbound: &'a OutboundMode,
    pub rule: Option<&'a Rule>,
}
