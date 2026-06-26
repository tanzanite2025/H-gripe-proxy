use anyhow::{Result, bail};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use smartstring::alias::String;
use std::{
    collections::{BTreeMap, BTreeSet},
    net::{TcpListener as StdTcpListener, UdpSocket as StdUdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener as TokioTcpListener, TcpStream, UdpSocket as TokioUdpSocket},
    sync::oneshot,
    time::{Duration, timeout},
};

use crate::{
    config::Config,
    core::{
        dns_runtime::dns_default_runtime_shadow_evidence,
        manager::RunningMode,
        runtime_snapshot::{build_proxies_from_runtime_config, build_rules_from_runtime_config},
    },
};

const MIHOMO_RUNTIME_ID: &str = "mihomo-kernel-runtime";
pub(super) const RUST_RUNTIME_ID: &str = "rust-kernel-runtime";
const NEXT_SAFE_BATCH: &str = "rust-shadow-components";
const NEXT_SHADOW_BATCH: &str = "loopback-test-listener-opt-in";
const ISOLATED_TEST_LISTENER_HOST: &str = "127.0.0.1";
const DEFAULT_ISOLATED_TEST_LISTENER_PORT: u16 = 19090;
const DEFAULT_LOOPBACK_DNS_PREFLIGHT_PORT: u16 = 19053;
const LOOPBACK_DNS_SMOKE_QUERY: &str = "kernel-smoke.invalid";
const DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT: u16 = 19180;
const DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT: u16 = 19181;
const LOOPBACK_PLATFORM_MATRIX_PLATFORMS: [&str; 3] = ["windows", "macos", "linux"];
const LOOPBACK_HOLD_WINDOW_MIN_SECONDS: u64 = 300;
const FULL_RUST_RUNTIME_HARDENING_MIN_SOAK_HOURS: u32 = 72;

static ISOLATED_TEST_LISTENER: Lazy<Mutex<Option<KernelIsolatedTestListenerState>>> = Lazy::new(|| Mutex::new(None));

mod data_plane_hardening;
mod default_data_plane_closeout;
mod encrypted_protocols_bundle;
mod encrypted_proxy_protocol;
mod encrypted_proxy_session;
mod fallback_retirement_execution;
mod fallback_retirement_readiness;
mod go_retirement;
mod http_connect_proxy_adapter;
mod loopback_cutover;
mod loopback_default_cutover;
mod loopback_migration;
mod operator_default_path_cutover;
mod protocol_adapter_forwarding;
mod protocol_forwarding;
mod remote_adapter_transport;
mod runtime_core;
mod runtime_real_canary;
mod rust_runtime_cutover;
mod shadowsocks_aead_adapter;
mod shadowsocks_aead_canary;
mod socks_auth_execution;
mod socks_bind_execution;
mod socks_tcp_connect_execution;
mod socks_udp_associate;
mod socks_udp_fragments;
mod tun_system_proxy;
mod tun_transparent_routing;
mod types;
pub use self::data_plane_hardening::*;
pub use self::default_data_plane_closeout::*;
pub use self::encrypted_protocols_bundle::*;
pub use self::encrypted_proxy_protocol::*;
pub use self::encrypted_proxy_session::*;
pub use self::fallback_retirement_execution::*;
pub use self::fallback_retirement_readiness::*;
pub use self::go_retirement::*;
pub use self::http_connect_proxy_adapter::*;
pub use self::loopback_cutover::*;
pub use self::loopback_default_cutover::*;
pub use self::loopback_migration::*;
pub use self::operator_default_path_cutover::*;
pub use self::protocol_adapter_forwarding::*;
pub use self::protocol_forwarding::*;
pub use self::remote_adapter_transport::*;
pub use self::runtime_core::*;
pub use self::runtime_real_canary::*;
pub use self::rust_runtime_cutover::*;
pub use self::shadowsocks_aead_adapter::*;
pub use self::shadowsocks_aead_canary::*;
pub use self::socks_auth_execution::*;
pub use self::socks_bind_execution::*;
pub use self::socks_tcp_connect_execution::*;
pub use self::socks_udp_associate::*;
pub use self::socks_udp_fragments::*;
pub use self::tun_system_proxy::*;
pub use self::tun_transparent_routing::*;
use self::types::KernelIsolatedTestListenerState;
pub use self::types::*;
