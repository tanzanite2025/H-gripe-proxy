use anyhow::Result;
use smartstring::alias::String;

pub type CmdResult<T = ()> = Result<T, String>;

// Command modules
pub mod anti_probe;
pub mod app;
pub mod backup;
pub mod blackhole_breaker;
pub mod china_rules;
pub mod clash;
pub mod coordinator;
pub mod dns;
pub mod egress_identity;
pub mod egress_monitor;
pub mod http;
pub mod identity_consistency;
pub mod ip_reputation;
pub mod media_unlock_checker;
pub mod multipath;
pub mod network;
pub mod profile;
pub mod proxy;
pub mod rule_engine;
pub mod runtime;
pub mod save_profile;
pub mod security;
pub mod security_policy;
pub mod service;
pub mod session_affinity;
pub mod system;
pub mod timezone_spoof;
pub mod tls_fingerprint;
pub mod tor;
pub mod traffic;
pub mod uwp;
pub mod validate;
pub mod verge;
pub mod webdav;
#[cfg(target_os = "linux")]
pub mod xdp;

// Re-export all command functions for backwards compatibility
pub use anti_probe::*;
pub use app::*;
pub use backup::*;
pub use blackhole_breaker::*;
pub use china_rules::*;
pub use clash::*;
pub use coordinator::*;
pub use dns::*;
pub use egress_identity::*;
pub use egress_monitor::*;
#[allow(unused_imports)]
pub use http::*;
pub use identity_consistency::*;
pub use ip_reputation::*;
pub use media_unlock_checker::*;
pub use multipath::*;
pub use network::*;
pub use profile::*;
pub use proxy::*;
pub use rule_engine::*;
pub use runtime::*;
pub use save_profile::*;
pub use security::*;
pub use security_policy::*;
pub use service::*;
pub use session_affinity::*;
pub use system::*;
pub use timezone_spoof::*;
pub use tls_fingerprint::*;
pub use tor::*;
#[allow(unused_imports)]
pub use traffic::*;
pub use uwp::*;
pub use validate::*;
pub use verge::*;
pub use webdav::*;
#[cfg(target_os = "linux")]
pub use xdp::*;

pub trait StringifyErr<T> {
    fn stringify_err(self) -> CmdResult<T>;
}

impl<T, E: std::fmt::Display> StringifyErr<T> for Result<T, E> {
    fn stringify_err(self) -> CmdResult<T> {
        self.map_err(|e| e.to_string().into())
    }
}
