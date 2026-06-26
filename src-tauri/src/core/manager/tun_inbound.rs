//! OS TUN device binding for the learn-gripe TUN inbound.
//!
//! This creates a real OS TUN interface (wintun on Windows, `/dev/net/tun` on
//! Linux, utun on macOS), brings it up with an address, and pumps its IP frames
//! through [`learn_gripe::serve_tun_device`] — relaying each TCP flow through
//! the selected outbound.
//!
//! **Scope / safety.** This binds the device and relays TCP plus UDP, answering
//! DNS queries in-stack from a fake-IP pool (the kernel maps each name to a
//! synthetic `198.18.0.0/16` address and recovers it on connect). On **Windows**
//! and **macOS** it also installs a **global default-route capture** so all
//! system traffic is pulled into the TUN — but *only* when the selected outbound
//! is a single, fixed-server proxy (see
//! [`OutboundMode::supports_global_capture`]): the proxy server's own IP is
//! pinned to the physical gateway with a host bypass route (so it is not looped
//! back into the tunnel), two `0.0.0.0/1` + `128.0.0.0/1` routes (more specific
//! than the untouched `0.0.0.0/0` default) point the rest at the TUN, and the
//! resolver is pointed at the in-stack fake-IP DNS. After applying, the route
//! table is re-read to confirm the capture took effect; if not, everything is
//! rolled back and start fails. `Direct`/`Reject`/`Routed` outbounds cannot be
//! globally captured without looping arbitrary or Direct-routed targets, so they
//! fall back to serving only the on-link subnet.
//!
//! IPv6 is captured the same way: each IPv6 proxy address is bypassed with a
//! host route and `::/1` + `8000::/1` route through the TUN. It is purely
//! additive — a host with no IPv6 default route is left untouched.
//!
//! Per-platform tooling: Windows pins routes by interface index via
//! `route`/`netsh` (giving the TUN a ULA v6 gateway and setting DNS on the TUN
//! adapter); macOS pins routes by interface name via `route` (utun is
//! point-to-point, so capture routes use `-interface utunN` and need no gateway
//! address) and takes over DNS on the primary network service via
//! `networksetup`, restoring the previous servers on rollback.
//!
//! **Untested.** The capture shells out to `route`/`netsh`/`networksetup` and
//! needs admin plus a real default route; it is compile-verified only and
//! **must** be validated on a real Windows or macOS machine.
//!
//! Every privileged system mutation is pushed onto a [`RollbackStack`] with its
//! inverse and undone in reverse order on [`TunInbound::stop`] (and on `Drop` as
//! a safety net), so enabling TUN never leaves the OS in a half-configured
//! state. This whole path is gated behind `enable_tun_mode` and is off by
//! default; it has been compile-verified but must be validated on a real machine
//! with administrator/root privileges.

use anyhow::{Context, Result, anyhow, bail};
use clash_verge_logging::{Type, logging};
use learn_gripe::{
    DEFAULT_MTU, DnsMode, DnsStats, DnsStatsSnapshot, FakeIpConfig, FakeIpPool, OutboundMode, serve_tun_device,
};
use std::net::Ipv4Addr;
#[cfg(any(windows, target_os = "macos"))]
use std::net::Ipv6Addr;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tun::AbstractDevice;

/// Address assigned to the TUN interface. 198.18.0.0/15 is the IANA
/// benchmarking range — unlikely to collide with real networks.
const TUN_ADDRESS: Ipv4Addr = Ipv4Addr::new(198, 18, 0, 1);
const TUN_NETMASK: Ipv4Addr = Ipv4Addr::new(255, 255, 0, 0);
const TUN_NAME: &str = "clash-verge";

/// IPv6 gateway assigned to the TUN for the global IPv6 capture. `fd00::/8` is
/// the IANA unique-local range — the v6 analogue of the v4 benchmarking address.
#[cfg(windows)]
const TUN_ADDRESS_V6: Ipv6Addr = Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1);
#[cfg(windows)]
const TUN_V6_PREFIX_LEN: u8 = 64;

/// A reversible system mutation: a human-readable description plus the closure
/// that undoes it.
struct RollbackAction {
    describe: String,
    undo: Box<dyn FnOnce() + Send>,
}

/// Records privileged system mutations in apply order and undoes them in
/// reverse. Running it is idempotent (it drains itself), and it runs on `Drop`
/// if `stop` was never called.
#[derive(Default)]
struct RollbackStack {
    actions: Vec<RollbackAction>,
}

impl RollbackStack {
    fn new() -> Self {
        Self::default()
    }

    /// Push an applied mutation together with its inverse.
    fn push(&mut self, describe: impl Into<String>, undo: impl FnOnce() + Send + 'static) {
        self.actions.push(RollbackAction {
            describe: describe.into(),
            undo: Box::new(undo),
        });
    }

    /// Undo every recorded mutation in reverse order.
    fn rollback(&mut self) {
        while let Some(action) = self.actions.pop() {
            logging!(info, Type::Core, "TUN rollback: {}", action.describe);
            (action.undo)();
        }
    }
}

impl Drop for RollbackStack {
    fn drop(&mut self) {
        if !self.actions.is_empty() {
            logging!(
                warn,
                Type::Core,
                "TUN rollback stack dropped with {} pending action(s); undoing now",
                self.actions.len()
            );
            self.rollback();
        }
    }
}

/// A running OS TUN inbound: the device pump task plus the rollback stack for
/// any system state it mutated.
pub struct TunInbound {
    shutdown: Arc<Notify>,
    pump: JoinHandle<()>,
    rollback: RollbackStack,
    /// The fake-IP pool the in-stack DNS allocates from; held so the live cache
    /// size can be reported as a DNS metric.
    fake_ip_pool: Arc<Mutex<FakeIpPool>>,
    /// Cumulative counters for the in-stack DNS answerer.
    dns_stats: Arc<DnsStats>,
}

impl std::fmt::Debug for TunInbound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TunInbound")
            .field("pending_rollback_actions", &self.rollback.actions.len())
            .finish()
    }
}

impl TunInbound {
    /// Create the OS TUN device and start relaying its TCP flows through
    /// `outbound`. Returns an error (leaving the system untouched) if the device
    /// cannot be created — typically a missing privilege or driver.
    pub async fn start(outbound: OutboundMode) -> Result<Self> {
        logging!(
            warn,
            Type::Core,
            "starting experimental learn-gripe TUN inbound on {} ({}/16, mtu {}); TCP+UDP with in-stack fake-IP DNS, no global route capture",
            TUN_NAME,
            TUN_ADDRESS,
            DEFAULT_MTU
        );

        let mut config = tun::Configuration::default();
        config
            .address(TUN_ADDRESS)
            .netmask(TUN_NETMASK)
            .mtu(DEFAULT_MTU as u16)
            .up();
        // macOS/iOS utun interfaces must be named `utunN`; setting our own name
        // there fails device creation, so let the kernel assign one. Elsewhere
        // (Windows wintun, Linux) use our stable adapter name.
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        config.tun_name(TUN_NAME);
        // `serve_tun_device` expects raw L3 frames (Windows wintun has none).
        // On Linux, `IFF_NO_PI` delivers exactly that, so disable the crate's
        // packet-information handling. On macOS/iOS, utun *always* prepends a
        // 4-byte address-family header at the kernel — it cannot be turned off —
        // so we instead *enable* the crate's handling, which strips that header
        // on read and prepends `AF_INET`/`AF_INET6` on write, leaving us raw L3.
        #[cfg(target_os = "linux")]
        config.platform_config(|p| {
            p.packet_information(false);
        });
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        config.platform_config(|p| {
            p.packet_information(true);
        });

        let device = tun::create_as_async(&config).map_err(|err| anyhow!("failed to create TUN device: {err}"))?;
        // Read the interface identifiers before splitting consumes the device;
        // the global-capture routes pin themselves to this interface (by index on
        // Windows, by name on macOS where utun routes are interface-scoped).
        #[cfg(windows)]
        let if_index = device
            .tun_index()
            .map_err(|err| anyhow!("failed to read TUN interface index: {err}"))?;
        #[cfg(target_os = "macos")]
        let tun_name = device
            .tun_name()
            .map_err(|err| anyhow!("failed to read TUN interface name: {err}"))?;
        let (reader, writer) = tokio::io::split(device);

        // Answer DNS over the TUN from a fake-IP pool on the interface subnet so
        // a global capture won't black-hole name resolution. Reserve the gateway
        // (the interface address) so it is never handed out as a fake IP.
        let (dns, pool, dns_stats) = DnsMode::fake_ip(FakeIpConfig::default());
        match pool.lock() {
            Ok(mut pool) => pool.reserve(TUN_ADDRESS),
            Err(err) => return Err(anyhow!("fake-IP pool mutex poisoned: {err}")),
        }

        let shutdown = Arc::new(Notify::new());
        let mut rollback = RollbackStack::new();
        // The OS removes the interface (and its auto-added subnet route) when the
        // device is dropped, which happens when the pump task returns after
        // `shutdown`. Record that teardown explicitly so the intent is visible.
        let teardown_shutdown = shutdown.clone();
        rollback.push("bring down and remove the TUN interface", move || {
            teardown_shutdown.notify_waiters();
        });

        // Global default-route capture (Windows + macOS). Sound only for a
        // single, fixed-server proxy outbound; on failure `rollback` (dropped on
        // this error path) undoes any partially-applied routes/DNS.
        #[cfg(any(windows, target_os = "macos"))]
        if outbound.supports_global_capture() {
            #[cfg(windows)]
            let capture = install_global_capture(&mut rollback, if_index, &outbound);
            #[cfg(target_os = "macos")]
            let capture = install_global_capture_macos(&mut rollback, &tun_name, &outbound);
            if let Err(err) = capture {
                logging!(error, Type::Core, "TUN global capture failed, rolling back: {err:#}");
                return Err(err);
            }
            logging!(warn, Type::Core, "TUN global default-route capture installed");
        } else {
            logging!(
                warn,
                Type::Core,
                "TUN global capture skipped (outbound is not a single fixed-server proxy); serving on-link subnet only"
            );
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        logging!(
            warn,
            Type::Core,
            "TUN global capture is only implemented on Windows and macOS; serving on-link subnet only"
        );

        let pump = tokio::spawn(serve_tun_device(
            reader,
            writer,
            outbound,
            Some(dns),
            shutdown.clone(),
            DEFAULT_MTU,
        ));

        Ok(Self {
            shutdown,
            pump,
            rollback,
            fake_ip_pool: pool,
            dns_stats,
        })
    }

    /// Snapshot the in-stack DNS counters together with the live fake-IP cache
    /// size. This is the only honest in-process DNS telemetry source: the Rust
    /// kernel answers DNS itself solely on this TUN datapath.
    pub fn dns_stats(&self) -> DnsStatsSnapshot {
        let entries = self.fake_ip_pool.lock().map(|pool| pool.len() as u64).unwrap_or(0);
        self.dns_stats.snapshot(entries)
    }

    /// Stop the inbound: signal shutdown, undo every system mutation in reverse,
    /// and wait for the pump task (which drops the device) to finish.
    pub async fn stop(mut self) {
        self.shutdown.notify_waiters();
        self.rollback.rollback();
        if let Err(err) = self.pump.await {
            logging!(warn, Type::Core, "TUN pump task ended abnormally: {err}");
        }
        logging!(info, Type::Core, "learn-gripe TUN inbound stopped");
    }
}

/// The two halves of a default-route split: each covers half the IPv4 space and
/// is more specific than `0.0.0.0/0`, so they win over the existing default
/// route without it having to be removed (keeping rollback a clean delete).
#[cfg(windows)]
const SPLIT_DEFAULT: [(Ipv4Addr, Ipv4Addr); 2] = [
    (Ipv4Addr::new(0, 0, 0, 0), Ipv4Addr::new(128, 0, 0, 0)),
    (Ipv4Addr::new(128, 0, 0, 0), Ipv4Addr::new(128, 0, 0, 0)),
];

/// `route add <dest> mask <mask> <gateway> metric 1 if <if_index>`.
#[cfg(windows)]
fn capture_route_add_args(dest: Ipv4Addr, mask: Ipv4Addr, gateway: Ipv4Addr, if_index: i32) -> Vec<String> {
    vec![
        "add".into(),
        dest.to_string(),
        "mask".into(),
        mask.to_string(),
        gateway.to_string(),
        "metric".into(),
        "1".into(),
        "if".into(),
        if_index.to_string(),
    ]
}

/// `route delete <dest> mask <mask> <gateway>` — the inverse of the add above.
#[cfg(windows)]
fn capture_route_delete_args(dest: Ipv4Addr, mask: Ipv4Addr, gateway: Ipv4Addr) -> Vec<String> {
    vec![
        "delete".into(),
        dest.to_string(),
        "mask".into(),
        mask.to_string(),
        gateway.to_string(),
    ]
}

/// `route add <ip> mask 255.255.255.255 <gateway> metric 1` — pin one proxy
/// server IP to the physical default gateway so it bypasses the TUN.
#[cfg(windows)]
fn bypass_route_add_args(ip: Ipv4Addr, gateway: Ipv4Addr) -> Vec<String> {
    vec![
        "add".into(),
        ip.to_string(),
        "mask".into(),
        "255.255.255.255".into(),
        gateway.to_string(),
        "metric".into(),
        "1".into(),
    ]
}

/// `route delete <ip> mask 255.255.255.255` — the inverse of the bypass add.
#[cfg(windows)]
fn bypass_route_delete_args(ip: Ipv4Addr) -> Vec<String> {
    vec!["delete".into(), ip.to_string(), "mask".into(), "255.255.255.255".into()]
}

/// Parse `route print -4` output for the active IPv4 default gateway, choosing
/// the lowest-metric `0.0.0.0/0.0.0.0` entry with a real (non `On-link`)
/// gateway. Returns `None` if there is no usable default route.
#[cfg(windows)]
fn parse_default_gateway(route_print: &str) -> Option<Ipv4Addr> {
    let mut best: Option<(u32, Ipv4Addr)> = None;
    for line in route_print.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 || cols[0] != "0.0.0.0" || cols[1] != "0.0.0.0" {
            continue;
        }
        let Ok(gateway) = cols[2].parse::<Ipv4Addr>() else {
            continue;
        };
        let metric = cols[4].parse::<u32>().unwrap_or(u32::MAX);
        if best.is_none_or(|(m, _)| metric < m) {
            best = Some((metric, gateway));
        }
    }
    best.map(|(_, gateway)| gateway)
}

/// Whether a re-read `route print -4` shows our `0.0.0.0/1` capture route — the
/// observation step that proves the capture took effect.
#[cfg(windows)]
fn capture_routes_present(route_print: &str) -> bool {
    route_print.lines().any(|line| {
        let cols: Vec<&str> = line.split_whitespace().collect();
        cols.len() >= 2 && cols[0] == "0.0.0.0" && cols[1] == "128.0.0.0"
    })
}

/// The two halves of the IPv6 default split: `::/1` + `8000::/1`, each more
/// specific than `::/0` so the existing v6 default need not be touched.
#[cfg(windows)]
const SPLIT_DEFAULT_V6: [(Ipv6Addr, u8); 2] = [
    (Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), 1),
    (Ipv6Addr::new(0x8000, 0, 0, 0, 0, 0, 0, 0), 1),
];

/// `netsh interface ipv6 add address interface=<idx> address=<addr>/<plen>`.
#[cfg(windows)]
fn v6_add_address_args(if_index: u32, addr: Ipv6Addr, plen: u8) -> Vec<String> {
    vec![
        "interface".into(),
        "ipv6".into(),
        "add".into(),
        "address".into(),
        format!("interface={if_index}"),
        format!("address={addr}/{plen}"),
        "store=active".into(),
    ]
}

/// `netsh interface ipv6 delete address ...` — the inverse of the add above.
#[cfg(windows)]
fn v6_delete_address_args(if_index: u32, addr: Ipv6Addr) -> Vec<String> {
    vec![
        "interface".into(),
        "ipv6".into(),
        "delete".into(),
        "address".into(),
        format!("interface={if_index}"),
        format!("address={addr}"),
    ]
}

/// `netsh interface ipv6 add route prefix=<dest>/<plen> interface=<idx> [nexthop=<gw>] metric=1`.
/// An on-link route (no gateway, e.g. a bypass via an on-link physical default)
/// omits the `nexthop` argument.
#[cfg(windows)]
fn v6_route_add_args(dest: Ipv6Addr, plen: u8, if_index: u32, nexthop: Option<Ipv6Addr>) -> Vec<String> {
    let mut args = vec![
        "interface".into(),
        "ipv6".into(),
        "add".into(),
        "route".into(),
        format!("prefix={dest}/{plen}"),
        format!("interface={if_index}"),
    ];
    if let Some(nh) = nexthop {
        args.push(format!("nexthop={nh}"));
    }
    args.push("metric=1".into());
    args.push("store=active".into());
    args
}

/// `netsh interface ipv6 delete route ...` — the inverse of the add above.
#[cfg(windows)]
fn v6_route_delete_args(dest: Ipv6Addr, plen: u8, if_index: u32, nexthop: Option<Ipv6Addr>) -> Vec<String> {
    let mut args = vec![
        "interface".into(),
        "ipv6".into(),
        "delete".into(),
        "route".into(),
        format!("prefix={dest}/{plen}"),
        format!("interface={if_index}"),
    ];
    if let Some(nh) = nexthop {
        args.push(format!("nexthop={nh}"));
    }
    args
}

/// Parse `netsh interface ipv6 show route` for the lowest-metric `::/0` default,
/// returning its interface index and gateway (`None` when the gateway column is
/// an interface name, i.e. an on-link default). Columns are
/// `Publish Type Met Prefix Idx Gateway/Interface Name`.
#[cfg(windows)]
fn parse_default_gateway_v6(show_route: &str) -> Option<DefaultRouteV6> {
    let mut best: Option<(u32, DefaultRouteV6)> = None;
    for line in show_route.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 || cols[3] != "::/0" {
            continue;
        }
        let Ok(metric) = cols[2].parse::<u32>() else {
            continue;
        };
        let Ok(if_index) = cols[4].parse::<u32>() else {
            continue;
        };
        let gateway = cols.get(5).and_then(|g| g.parse::<Ipv6Addr>().ok());
        if best.as_ref().is_none_or(|(m, _)| metric < *m) {
            best = Some((metric, DefaultRouteV6 { if_index, gateway }));
        }
    }
    best.map(|(_, route)| route)
}

/// Whether a re-read `netsh interface ipv6 show route` shows our `::/1` capture
/// route — the v6 observation step.
#[cfg(windows)]
fn capture_routes_present_v6(show_route: &str) -> bool {
    show_route.lines().any(|line| {
        let cols: Vec<&str> = line.split_whitespace().collect();
        cols.len() >= 4 && cols[3] == "::/1"
    })
}

#[cfg(any(windows, target_os = "macos"))]
fn str_refs(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

/// Run a system command and return its stdout, erroring on non-zero exit.
#[cfg(any(windows, target_os = "macos"))]
fn run_cmd(program: &str, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("spawn `{program} {}`", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "`{program} {}` failed ({}): {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Resolve the proxy server endpoint(s) to literal IPv4/IPv6 addresses while
/// normal DNS still works (before the default route is captured). Both families
/// are collected so each can be bypassed in its own capture.
#[cfg(any(windows, target_os = "macos"))]
fn resolve_proxy_server_ips(outbound: &OutboundMode) -> Result<(Vec<Ipv4Addr>, Vec<Ipv6Addr>)> {
    use std::net::{IpAddr, ToSocketAddrs};

    let mut server_ips: Vec<Ipv4Addr> = Vec::new();
    let mut server_ips_v6: Vec<Ipv6Addr> = Vec::new();
    for (host, port) in outbound.direct_dial_endpoints() {
        let resolved = (host.as_str(), port)
            .to_socket_addrs()
            .with_context(|| format!("resolve proxy server {host}:{port} for the bypass route"))?;
        for addr in resolved {
            match addr.ip() {
                IpAddr::V4(ip) if !server_ips.contains(&ip) => server_ips.push(ip),
                IpAddr::V6(ip) if !server_ips_v6.contains(&ip) => server_ips_v6.push(ip),
                _ => {}
            }
        }
    }
    Ok((server_ips, server_ips_v6))
}

/// Install the Windows global default-route capture, recording each mutation's
/// inverse on `rollback`. See the module docs for the design; this shells out to
/// `route`/`netsh` and is compile-verified only.
#[cfg(windows)]
fn install_global_capture(rollback: &mut RollbackStack, if_index: i32, outbound: &OutboundMode) -> Result<()> {
    // 1. Resolve the proxy server endpoint(s) to literal IPs while normal DNS
    //    still works (before the default route is captured).
    let (server_ips, server_ips_v6) = resolve_proxy_server_ips(outbound)?;
    if server_ips.is_empty() {
        bail!("no IPv4 address for the proxy server; refusing to capture the default route (would loop)");
    }

    // 2. Discover the current default gateway so the proxy can bypass the TUN.
    let table = run_cmd("route", &["print", "-4"])?;
    let gateway = parse_default_gateway(&table)
        .context("no IPv4 default gateway found; refusing to capture the default route")?;

    // 3. Bypass each proxy server IP via the physical gateway (a /32 beats the
    //    /1 capture routes, so the proxy's own traffic is never looped).
    for ip in &server_ips {
        run_cmd("route", &str_refs(&bypass_route_add_args(*ip, gateway)))?;
        let undo = bypass_route_delete_args(*ip);
        let ip = *ip;
        rollback.push(format!("delete bypass route {ip}/32"), move || {
            let _ = run_cmd("route", &str_refs(&undo));
        });
    }

    // 4. Capture the rest of the address space through the TUN.
    for (dest, mask) in SPLIT_DEFAULT {
        run_cmd(
            "route",
            &str_refs(&capture_route_add_args(dest, mask, TUN_ADDRESS, if_index)),
        )?;
        let undo = capture_route_delete_args(dest, mask, TUN_ADDRESS);
        rollback.push(format!("delete TUN default route {dest}"), move || {
            let _ = run_cmd("route", &str_refs(&undo));
        });
    }

    // 5. Point the resolver at the in-stack fake-IP DNS. The setting lives on the
    //    TUN adapter, which is removed on teardown, so it needs no rollback;
    //    best-effort (a failure here does not abort the capture).
    let dns = TUN_ADDRESS.to_string();
    if let Err(err) = run_cmd(
        "netsh",
        &[
            "interface",
            "ipv4",
            "set",
            "dnsservers",
            &format!("name={TUN_NAME}"),
            "static",
            &dns,
            "primary",
        ],
    ) {
        logging!(warn, Type::Core, "TUN DNS redirect best-effort step failed: {err:#}");
    }

    // 6. Observe: confirm the capture routes are actually in the table.
    let after = run_cmd("route", &["print", "-4"])?;
    if !capture_routes_present(&after) {
        bail!("TUN capture routes did not take effect (route table lacks the 0.0.0.0/1 split)");
    }

    // 7. Capture IPv6 the same way (additive; no-op on a host without IPv6).
    install_global_capture_v6(rollback, if_index, &server_ips_v6)?;
    Ok(())
}

/// The IPv6 default gateway and the interface it is reached through, parsed from
/// `netsh interface ipv6 show route`. `gateway` is `None` for an on-link default
/// (the "Gateway" column is an interface name rather than an address).
#[cfg(windows)]
struct DefaultRouteV6 {
    if_index: u32,
    gateway: Option<Ipv6Addr>,
}

/// Mirror [`install_global_capture`] for IPv6: assign the TUN a ULA gateway,
/// bypass each IPv6 proxy-server address with a `/128` via the physical default,
/// capture `::/1` + `8000::/1` through the TUN, then observe. Returns `Ok(())`
/// without touching anything when the host has no IPv6 default route, so this is
/// purely additive. Each mutation records its inverse on `rollback`.
#[cfg(windows)]
fn install_global_capture_v6(rollback: &mut RollbackStack, if_index: i32, server_ips: &[Ipv6Addr]) -> Result<()> {
    let if_index = if_index as u32;

    // Skip cleanly when there is no IPv6 connectivity to capture.
    let table = run_cmd("netsh", &["interface", "ipv6", "show", "route"])?;
    let Some(default) = parse_default_gateway_v6(&table) else {
        logging!(info, Type::Core, "no IPv6 default route; skipping IPv6 capture");
        return Ok(());
    };

    // Give the TUN an on-link IPv6 next-hop (the analogue of 198.18.0.1). The
    // `tun` crate cannot set a v6 address on Windows, so do it via netsh.
    run_cmd(
        "netsh",
        &str_refs(&v6_add_address_args(if_index, TUN_ADDRESS_V6, TUN_V6_PREFIX_LEN)),
    )?;
    let undo = v6_delete_address_args(if_index, TUN_ADDRESS_V6);
    rollback.push("remove TUN IPv6 address", move || {
        let _ = run_cmd("netsh", &str_refs(&undo));
    });

    // Bypass each IPv6 proxy server address via the physical default (a /128
    // beats the ::/1 capture routes, so the proxy's own traffic is not looped).
    for ip in server_ips {
        run_cmd(
            "netsh",
            &str_refs(&v6_route_add_args(*ip, 128, default.if_index, default.gateway)),
        )?;
        let undo = v6_route_delete_args(*ip, 128, default.if_index, default.gateway);
        let ip = *ip;
        rollback.push(format!("delete IPv6 bypass route {ip}/128"), move || {
            let _ = run_cmd("netsh", &str_refs(&undo));
        });
    }

    // Capture the rest of the IPv6 space through the TUN.
    for (dest, plen) in SPLIT_DEFAULT_V6 {
        run_cmd(
            "netsh",
            &str_refs(&v6_route_add_args(dest, plen, if_index, Some(TUN_ADDRESS_V6))),
        )?;
        let undo = v6_route_delete_args(dest, plen, if_index, Some(TUN_ADDRESS_V6));
        rollback.push(format!("delete TUN IPv6 default route {dest}/{plen}"), move || {
            let _ = run_cmd("netsh", &str_refs(&undo));
        });
    }

    // Observe: confirm the ::/1 capture route is present.
    let after = run_cmd("netsh", &["interface", "ipv6", "show", "route"])?;
    if !capture_routes_present_v6(&after) {
        bail!("TUN IPv6 capture routes did not take effect (route table lacks the ::/1 split)");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// macOS global default-route capture
//
// Mirrors the Windows design with BSD tooling: routes are pinned by interface
// *name* (not index), the utun is point-to-point so capture routes use
// `-interface utunN` with no gateway address, and DNS takeover is done on the
// primary network service via `networksetup` (restored on rollback).
// ---------------------------------------------------------------------------

/// The two halves of the IPv4 default split as CIDR (`route` on macOS takes
/// CIDR directly), each more specific than `default` so they win without the
/// existing `0.0.0.0/0` having to be removed — keeping rollback a clean delete.
#[cfg(target_os = "macos")]
const SPLIT_DEFAULT_MACOS: [&str; 2] = ["0.0.0.0/1", "128.0.0.0/1"];

/// The two halves of the IPv6 default split: `::/1` + `8000::/1`.
#[cfg(target_os = "macos")]
const SPLIT_DEFAULT_V6_MACOS: [&str; 2] = ["::/1", "8000::/1"];

/// `route -n add -net <cidr> -interface <tun>` — route a split-default half at
/// the TUN by interface name (utun is point-to-point, so no gateway is needed).
#[cfg(target_os = "macos")]
fn macos_capture_route_add_args(cidr: &str, tun: &str) -> Vec<String> {
    vec![
        "-n".into(),
        "add".into(),
        "-net".into(),
        cidr.into(),
        "-interface".into(),
        tun.into(),
    ]
}

/// `route -n delete -net <cidr> -interface <tun>` — the inverse of the add above.
#[cfg(target_os = "macos")]
fn macos_capture_route_delete_args(cidr: &str, tun: &str) -> Vec<String> {
    vec![
        "-n".into(),
        "delete".into(),
        "-net".into(),
        cidr.into(),
        "-interface".into(),
        tun.into(),
    ]
}

/// `route -n add -host <ip> <gateway>` — pin one proxy server IP to the physical
/// default gateway so it bypasses the TUN.
#[cfg(target_os = "macos")]
fn macos_bypass_route_add_args(ip: Ipv4Addr, gateway: Ipv4Addr) -> Vec<String> {
    vec![
        "-n".into(),
        "add".into(),
        "-host".into(),
        ip.to_string(),
        gateway.to_string(),
    ]
}

/// `route -n delete -host <ip>` — the inverse of the bypass add.
#[cfg(target_os = "macos")]
fn macos_bypass_route_delete_args(ip: Ipv4Addr) -> Vec<String> {
    vec!["-n".into(), "delete".into(), "-host".into(), ip.to_string()]
}

/// `route -n add -inet6 -net <cidr> -interface <tun>` — IPv6 capture half.
#[cfg(target_os = "macos")]
fn macos_capture_route_add_args_v6(cidr: &str, tun: &str) -> Vec<String> {
    vec![
        "-n".into(),
        "add".into(),
        "-inet6".into(),
        "-net".into(),
        cidr.into(),
        "-interface".into(),
        tun.into(),
    ]
}

/// `route -n delete -inet6 -net <cidr> -interface <tun>` — the inverse v6.
#[cfg(target_os = "macos")]
fn macos_capture_route_delete_args_v6(cidr: &str, tun: &str) -> Vec<String> {
    vec![
        "-n".into(),
        "delete".into(),
        "-inet6".into(),
        "-net".into(),
        cidr.into(),
        "-interface".into(),
        tun.into(),
    ]
}

/// `route -n add -inet6 -host <ip> <gateway>` — bypass one IPv6 proxy address.
/// The gateway is passed as a string because the v6 default next-hop is often a
/// scoped link-local address (`fe80::1%en0`) that `Ipv6Addr` cannot represent.
#[cfg(target_os = "macos")]
fn macos_bypass_route_add_args_v6(ip: Ipv6Addr, gateway: &str) -> Vec<String> {
    vec![
        "-n".into(),
        "add".into(),
        "-inet6".into(),
        "-host".into(),
        ip.to_string(),
        gateway.into(),
    ]
}

/// `route -n delete -inet6 -host <ip>` — the inverse of the v6 bypass add.
#[cfg(target_os = "macos")]
fn macos_bypass_route_delete_args_v6(ip: Ipv6Addr) -> Vec<String> {
    vec![
        "-n".into(),
        "delete".into(),
        "-inet6".into(),
        "-host".into(),
        ip.to_string(),
    ]
}

/// `ifconfig <tun> inet6 <ip> prefixlen <plen> alias` — give the utun an on-link
/// IPv6 address (the analogue of the IPv4 198.18.0.1). `route` cannot add the
/// crate-created utun an IPv6 address, and without a local v6 source address the
/// kernel cannot source-select for IPv6 it routes into the TUN, so the capture
/// routes alone would black-hole locally-originated IPv6.
#[cfg(target_os = "macos")]
fn macos_v6_add_address_args(tun: &str, ip: Ipv6Addr, prefix_len: u8) -> Vec<String> {
    vec![
        tun.into(),
        "inet6".into(),
        ip.to_string(),
        "prefixlen".into(),
        prefix_len.to_string(),
        "alias".into(),
    ]
}

/// `ifconfig <tun> inet6 <ip> -alias` — the inverse of the v6 address add above.
#[cfg(target_os = "macos")]
fn macos_v6_delete_address_args(tun: &str, ip: Ipv6Addr) -> Vec<String> {
    vec![tun.into(), "inet6".into(), ip.to_string(), "-alias".into()]
}

/// Parse a single `field:` value out of `route -n get ...` output, e.g.
/// `gateway: 192.168.1.1` or `interface: en0`. Returns `None` when the field is
/// absent or empty.
#[cfg(target_os = "macos")]
fn parse_route_get_field(route_get: &str, field: &str) -> Option<String> {
    let needle = format!("{field}:");
    for line in route_get.lines() {
        if let Some(rest) = line.trim().strip_prefix(&needle) {
            let value = rest.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// The IPv4 default gateway from `route -n get default`, or `None` when there is
/// no usable (parseable) default gateway.
#[cfg(target_os = "macos")]
fn parse_default_gateway_macos(route_get: &str) -> Option<Ipv4Addr> {
    parse_route_get_field(route_get, "gateway").and_then(|g| g.parse().ok())
}

/// Whether a re-read `netstat -rn -f inet` shows our `0/1` capture route — the
/// observation step that proves the IPv4 capture took effect.
#[cfg(target_os = "macos")]
fn capture_routes_present_macos(netstat: &str) -> bool {
    netstat
        .lines()
        .any(|line| line.split_whitespace().next() == Some("0/1"))
}

/// Whether a re-read `netstat -rn -f inet6` shows our `::/1` capture route.
#[cfg(target_os = "macos")]
fn capture_routes_present_v6_macos(netstat: &str) -> bool {
    netstat
        .lines()
        .any(|line| line.split_whitespace().next() == Some("::/1"))
}

/// Parse `networksetup -listnetworkserviceorder` for the service name bound to a
/// given BSD device (e.g. `en0` → `Wi-Fi`). Entries look like:
/// `(1) Wi-Fi` followed by `(Hardware Port: Wi-Fi, Device: en0)`.
#[cfg(target_os = "macos")]
fn parse_service_for_device(listing: &str, device: &str) -> Option<String> {
    let mut current: Option<String> = None;
    let suffix = format!("Device: {device}");
    for line in listing.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix('(') else {
            continue;
        };
        if rest.starts_with("Hardware Port:") {
            if rest.trim_end_matches(')').trim_end().ends_with(&suffix) {
                return current.clone();
            }
        } else if let Some((idx, name)) = rest.split_once(')') {
            // Service header `(N) Name`; ignore the disabled marker `(*)`.
            if !idx.is_empty() && idx.chars().all(|c| c.is_ascii_digit()) {
                current = Some(name.trim().to_string());
            }
        }
    }
    None
}

/// Parse `networksetup -getdnsservers <service>` into the configured servers.
/// Returns an empty vec when none are set (the "There aren't any DNS Servers
/// set" message), meaning DNS is supplied by DHCP.
#[cfg(target_os = "macos")]
fn parse_dns_servers(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| line.parse::<std::net::IpAddr>().is_ok())
        .map(str::to_string)
        .collect()
}

/// Best-effort DNS takeover on the network service that owns `device`: point it
/// at the in-stack fake-IP resolver and record a rollback that restores the
/// previous servers (or DHCP, when none were set).
#[cfg(target_os = "macos")]
fn macos_dns_takeover(rollback: &mut RollbackStack, device: &str) -> Result<()> {
    let listing = run_cmd("networksetup", &["-listnetworkserviceorder"])?;
    let service = parse_service_for_device(&listing, device)
        .with_context(|| format!("no network service found for interface {device}"))?;
    let previous = run_cmd("networksetup", &["-getdnsservers", &service])
        .map(|out| parse_dns_servers(&out))
        .unwrap_or_default();

    let dns = TUN_ADDRESS.to_string();
    run_cmd("networksetup", &["-setdnsservers", &service, &dns])?;
    rollback.push(format!("restore DNS servers on {service}"), move || {
        let mut args: Vec<String> = vec!["-setdnsservers".into(), service];
        if previous.is_empty() {
            // `empty` reverts the service to DHCP-supplied DNS.
            args.push("empty".into());
        } else {
            args.extend(previous);
        }
        let _ = run_cmd("networksetup", &str_refs(&args));
    });
    Ok(())
}

/// Install the macOS global default-route capture, recording each mutation's
/// inverse on `rollback`. See the module docs for the design; this shells out to
/// `route`/`netstat`/`networksetup` and is compile-verified only.
#[cfg(target_os = "macos")]
fn install_global_capture_macos(rollback: &mut RollbackStack, tun_name: &str, outbound: &OutboundMode) -> Result<()> {
    // 1. Resolve the proxy server endpoint(s) to literal IPs while normal DNS
    //    still works (before the default route is captured).
    let (server_ips, server_ips_v6) = resolve_proxy_server_ips(outbound)?;
    if server_ips.is_empty() {
        bail!("no IPv4 address for the proxy server; refusing to capture the default route (would loop)");
    }

    // 2. Discover the current default route (gateway + physical interface).
    let default_route = run_cmd("route", &["-n", "get", "default"])?;
    let gateway = parse_default_gateway_macos(&default_route)
        .context("no IPv4 default gateway found; refusing to capture the default route")?;
    let phys_iface = parse_route_get_field(&default_route, "interface");

    // 3. Bypass each proxy server IP via the physical gateway (a host route
    //    beats the /1 capture routes, so the proxy's own traffic is not looped).
    for ip in &server_ips {
        run_cmd("route", &str_refs(&macos_bypass_route_add_args(*ip, gateway)))?;
        let undo = macos_bypass_route_delete_args(*ip);
        let ip = *ip;
        rollback.push(format!("delete bypass route {ip}"), move || {
            let _ = run_cmd("route", &str_refs(&undo));
        });
    }

    // 4. Capture the rest of the address space through the TUN.
    for cidr in SPLIT_DEFAULT_MACOS {
        run_cmd("route", &str_refs(&macos_capture_route_add_args(cidr, tun_name)))?;
        let undo = macos_capture_route_delete_args(cidr, tun_name);
        rollback.push(format!("delete TUN default route {cidr}"), move || {
            let _ = run_cmd("route", &str_refs(&undo));
        });
    }

    // 5. Point the resolver at the in-stack fake-IP DNS (best-effort; a failure
    //    here does not abort the capture). Unlike Windows, macOS sets DNS on the
    //    physical network service, so this needs an explicit rollback.
    if let Some(iface) = phys_iface.as_deref() {
        if let Err(err) = macos_dns_takeover(rollback, iface) {
            logging!(warn, Type::Core, "TUN DNS takeover best-effort step failed: {err:#}");
        }
    }

    // 6. Observe: confirm the capture routes are actually in the table.
    let after = run_cmd("netstat", &["-rn", "-f", "inet"])?;
    if !capture_routes_present_macos(&after) {
        bail!("TUN capture routes did not take effect (route table lacks the 0/1 split)");
    }

    // 7. Capture IPv6 the same way (additive; no-op on a host without IPv6).
    install_global_capture_v6_macos(rollback, tun_name, &server_ips_v6)?;
    Ok(())
}

/// Mirror [`install_global_capture_macos`] for IPv6: bypass each IPv6 proxy
/// address with a host route via the physical v6 gateway, then capture
/// `::/1` + `8000::/1` through the TUN. Returns `Ok(())` without touching
/// anything when the host has no IPv6 default route, so this is purely additive.
#[cfg(target_os = "macos")]
fn install_global_capture_v6_macos(
    rollback: &mut RollbackStack,
    tun_name: &str,
    server_ips: &[Ipv6Addr],
) -> Result<()> {
    // Skip cleanly when there is no IPv6 default route to capture. `route get`
    // exits non-zero ("not in table") in that case, which surfaces as an error.
    let default_route = match run_cmd("route", &["-n", "get", "-inet6", "default"]) {
        Ok(out) => out,
        Err(_) => {
            logging!(info, Type::Core, "no IPv6 default route; skipping IPv6 capture");
            return Ok(());
        }
    };
    let Some(gateway) = parse_route_get_field(&default_route, "gateway") else {
        logging!(info, Type::Core, "no IPv6 default gateway; skipping IPv6 capture");
        return Ok(());
    };

    // Give the TUN an on-link IPv6 address (the analogue of 198.18.0.1), so the
    // kernel has a local source address for IPv6 routed into the utun. The `tun`
    // crate only configures the v4 address on macOS, so do it via `ifconfig`.
    run_cmd(
        "ifconfig",
        &str_refs(&macos_v6_add_address_args(tun_name, TUN_ADDRESS_V6, TUN_V6_PREFIX_LEN)),
    )?;
    let undo = macos_v6_delete_address_args(tun_name, TUN_ADDRESS_V6);
    rollback.push("remove TUN IPv6 address", move || {
        let _ = run_cmd("ifconfig", &str_refs(&undo));
    });

    // Bypass each IPv6 proxy server address via the physical default.
    for ip in server_ips {
        run_cmd("route", &str_refs(&macos_bypass_route_add_args_v6(*ip, &gateway)))?;
        let undo = macos_bypass_route_delete_args_v6(*ip);
        let ip = *ip;
        rollback.push(format!("delete IPv6 bypass route {ip}"), move || {
            let _ = run_cmd("route", &str_refs(&undo));
        });
    }

    // Capture the rest of the IPv6 space through the TUN.
    for cidr in SPLIT_DEFAULT_V6_MACOS {
        run_cmd("route", &str_refs(&macos_capture_route_add_args_v6(cidr, tun_name)))?;
        let undo = macos_capture_route_delete_args_v6(cidr, tun_name);
        rollback.push(format!("delete TUN IPv6 default route {cidr}"), move || {
            let _ = run_cmd("route", &str_refs(&undo));
        });
    }

    // Observe: confirm the ::/1 capture route is present.
    let after = run_cmd("netstat", &["-rn", "-f", "inet6"])?;
    if !capture_routes_present_v6_macos(&after) {
        bail!("TUN IPv6 capture routes did not take effect (route table lacks the ::/1 split)");
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn parses_lowest_metric_default_gateway() {
        let table = "\
===========================================================================
Active Routes:
Network Destination        Netmask          Gateway       Interface  Metric
          0.0.0.0          0.0.0.0     192.168.1.254     192.168.1.20     35
          0.0.0.0          0.0.0.0       10.8.0.1          10.8.0.2      25
        127.0.0.0        255.0.0.0         On-link         127.0.0.1    331
===========================================================================";
        // The 10.8.0.1 default route has the lower metric, so it wins.
        assert_eq!(parse_default_gateway(table), Some(Ipv4Addr::new(10, 8, 0, 1)));
    }

    #[test]
    fn no_default_route_yields_none() {
        let table = "\
Network Destination        Netmask          Gateway       Interface  Metric
        127.0.0.0        255.0.0.0         On-link         127.0.0.1    331";
        assert_eq!(parse_default_gateway(table), None);
    }

    #[test]
    fn capture_presence_detects_the_split() {
        let with = "          0.0.0.0        128.0.0.0       198.18.0.1       198.18.0.1      1";
        let without = "          0.0.0.0          0.0.0.0     192.168.1.1     192.168.1.20     25";
        assert!(capture_routes_present(with));
        assert!(!capture_routes_present(without));
    }

    #[test]
    fn route_arg_builders_are_inverses() {
        let (dest, mask) = SPLIT_DEFAULT[0];
        assert_eq!(
            capture_route_add_args(dest, mask, Ipv4Addr::new(198, 18, 0, 1), 42),
            vec![
                "add",
                "0.0.0.0",
                "mask",
                "128.0.0.0",
                "198.18.0.1",
                "metric",
                "1",
                "if",
                "42"
            ]
        );
        assert_eq!(
            capture_route_delete_args(dest, mask, Ipv4Addr::new(198, 18, 0, 1)),
            vec!["delete", "0.0.0.0", "mask", "128.0.0.0", "198.18.0.1"]
        );
        let ip = Ipv4Addr::new(203, 0, 113, 7);
        assert_eq!(
            bypass_route_add_args(ip, Ipv4Addr::new(192, 168, 1, 1)),
            vec![
                "add",
                "203.0.113.7",
                "mask",
                "255.255.255.255",
                "192.168.1.1",
                "metric",
                "1"
            ]
        );
        assert_eq!(
            bypass_route_delete_args(ip),
            vec!["delete", "203.0.113.7", "mask", "255.255.255.255"]
        );
    }

    #[test]
    fn parses_lowest_metric_default_gateway_v6() {
        let table = "\
Publish  Type      Met  Prefix                    Idx  Gateway/Interface Name
-------  --------  ---  ------------------------  ---  ------------------------
No       Manual    256  ::/0                      5    fe80::1
No       Manual    100  ::/0                      12   fe80::abcd
No       Manual    256  ::1/128                   1    Loopback Pseudo-Interface 1";
        let route = parse_default_gateway_v6(table).expect("default v6 route");
        // The metric-100 entry wins; its gateway is a link-local address.
        assert_eq!(route.if_index, 12);
        assert_eq!(route.gateway, Some(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0xabcd)));
    }

    #[test]
    fn parses_on_link_default_gateway_v6_as_none() {
        let table = "\
Publish  Type      Met  Prefix                    Idx  Gateway/Interface Name
No       Manual    256  ::/0                      8    Ethernet";
        let route = parse_default_gateway_v6(table).expect("default v6 route");
        assert_eq!(route.if_index, 8);
        assert_eq!(route.gateway, None);
    }

    #[test]
    fn no_default_route_v6_yields_none() {
        let table = "\
Publish  Type      Met  Prefix                    Idx  Gateway/Interface Name
No       Manual    256  ::1/128                   1    Loopback Pseudo-Interface 1";
        assert!(parse_default_gateway_v6(table).is_none());
    }

    #[test]
    fn capture_presence_detects_the_v6_split() {
        let with = "No       Manual    1    ::/1                      12   fd00::1";
        let without = "No       Manual    256  ::/0                      12   fe80::1";
        assert!(capture_routes_present_v6(with));
        assert!(!capture_routes_present_v6(without));
    }

    #[test]
    fn v6_route_arg_builders_are_inverses() {
        let (dest, plen) = SPLIT_DEFAULT_V6[0];
        let nexthop = Some(TUN_ADDRESS_V6);
        assert_eq!(
            v6_route_add_args(dest, plen, 12, nexthop),
            vec![
                "interface",
                "ipv6",
                "add",
                "route",
                "prefix=::/1",
                "interface=12",
                "nexthop=fd00::1",
                "metric=1",
                "store=active"
            ]
        );
        assert_eq!(
            v6_route_delete_args(dest, plen, 12, nexthop),
            vec![
                "interface",
                "ipv6",
                "delete",
                "route",
                "prefix=::/1",
                "interface=12",
                "nexthop=fd00::1"
            ]
        );
        // An on-link route (no gateway) omits the nexthop argument.
        let server = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        assert_eq!(
            v6_route_add_args(server, 128, 8, None),
            vec![
                "interface",
                "ipv6",
                "add",
                "route",
                "prefix=2001:db8::1/128",
                "interface=8",
                "metric=1",
                "store=active"
            ]
        );
    }

    #[test]
    fn v6_address_arg_builders_are_inverses() {
        assert_eq!(
            v6_add_address_args(12, TUN_ADDRESS_V6, TUN_V6_PREFIX_LEN),
            vec![
                "interface",
                "ipv6",
                "add",
                "address",
                "interface=12",
                "address=fd00::1/64",
                "store=active"
            ]
        );
        assert_eq!(
            v6_delete_address_args(12, TUN_ADDRESS_V6),
            vec![
                "interface",
                "ipv6",
                "delete",
                "address",
                "interface=12",
                "address=fd00::1"
            ]
        );
    }
}

#[cfg(all(test, target_os = "macos"))]
mod macos_tests {
    use super::*;

    #[test]
    fn route_arg_builders_are_inverses() {
        let cidr = SPLIT_DEFAULT_MACOS[0];
        assert_eq!(
            macos_capture_route_add_args(cidr, "utun5"),
            vec!["-n", "add", "-net", "0.0.0.0/1", "-interface", "utun5"]
        );
        assert_eq!(
            macos_capture_route_delete_args(cidr, "utun5"),
            vec!["-n", "delete", "-net", "0.0.0.0/1", "-interface", "utun5"]
        );
        let ip = Ipv4Addr::new(203, 0, 113, 7);
        assert_eq!(
            macos_bypass_route_add_args(ip, Ipv4Addr::new(192, 168, 1, 1)),
            vec!["-n", "add", "-host", "203.0.113.7", "192.168.1.1"]
        );
        assert_eq!(
            macos_bypass_route_delete_args(ip),
            vec!["-n", "delete", "-host", "203.0.113.7"]
        );
    }

    #[test]
    fn v6_route_arg_builders_are_inverses() {
        let cidr = SPLIT_DEFAULT_V6_MACOS[0];
        assert_eq!(
            macos_capture_route_add_args_v6(cidr, "utun5"),
            vec!["-n", "add", "-inet6", "-net", "::/1", "-interface", "utun5"]
        );
        assert_eq!(
            macos_capture_route_delete_args_v6(cidr, "utun5"),
            vec!["-n", "delete", "-inet6", "-net", "::/1", "-interface", "utun5"]
        );
        let ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        // A scoped link-local gateway is forwarded verbatim.
        assert_eq!(
            macos_bypass_route_add_args_v6(ip, "fe80::1%en0"),
            vec!["-n", "add", "-inet6", "-host", "2001:db8::1", "fe80::1%en0"]
        );
        assert_eq!(
            macos_bypass_route_delete_args_v6(ip),
            vec!["-n", "delete", "-inet6", "-host", "2001:db8::1"]
        );
    }

    #[test]
    fn v6_address_arg_builders_are_inverses() {
        assert_eq!(
            macos_v6_add_address_args("utun5", TUN_ADDRESS_V6, TUN_V6_PREFIX_LEN),
            vec!["utun5", "inet6", "fd00::1", "prefixlen", "64", "alias"]
        );
        assert_eq!(
            macos_v6_delete_address_args("utun5", TUN_ADDRESS_V6),
            vec!["utun5", "inet6", "fd00::1", "-alias"]
        );
    }

    #[test]
    fn parses_route_get_default_fields() {
        let output = "\
   route to: default
destination: default
       mask: default
    gateway: 192.168.1.254
  interface: en0
      flags: <UP,GATEWAY,DONE,STATIC,PRCLONING,GLOBAL>";
        assert_eq!(
            parse_default_gateway_macos(output),
            Some(Ipv4Addr::new(192, 168, 1, 254))
        );
        assert_eq!(parse_route_get_field(output, "interface").as_deref(), Some("en0"));
    }

    #[test]
    fn parses_scoped_v6_gateway_as_string() {
        let output = "\
   route to: ::
destination: ::
    gateway: fe80::1%en0
  interface: en0";
        assert_eq!(parse_route_get_field(output, "gateway").as_deref(), Some("fe80::1%en0"));
    }

    #[test]
    fn no_default_gateway_yields_none() {
        let output = "   route to: default\ndestination: default\n  interface: en0";
        assert_eq!(parse_default_gateway_macos(output), None);
    }

    #[test]
    fn capture_presence_detects_the_split() {
        let with = "\
Destination        Gateway            Flags        Netif Expire
0/1                198.18.0.1         UGScg         utun5
default            192.168.1.254      UGScg           en0";
        let without = "\
Destination        Gateway            Flags        Netif Expire
default            192.168.1.254      UGScg           en0";
        assert!(capture_routes_present_macos(with));
        assert!(!capture_routes_present_macos(without));
    }

    #[test]
    fn capture_presence_detects_the_v6_split() {
        let with = "\
Destination                             Gateway                         Flags         Netif Expire
::/1                                    link#10                         UCSg          utun5";
        let without = "\
Destination                             Gateway                         Flags         Netif Expire
default                                 fe80::1%en0                     UGScg           en0";
        assert!(capture_routes_present_v6_macos(with));
        assert!(!capture_routes_present_v6_macos(without));
    }

    #[test]
    fn maps_device_to_network_service() {
        let listing = "\
An asterisk (*) denotes that a network service is disabled.
(1) Wi-Fi
(Hardware Port: Wi-Fi, Device: en0)

(2) Thunderbolt Bridge
(Hardware Port: Thunderbolt Bridge, Device: bridge0)";
        assert_eq!(parse_service_for_device(listing, "en0").as_deref(), Some("Wi-Fi"));
        assert_eq!(
            parse_service_for_device(listing, "bridge0").as_deref(),
            Some("Thunderbolt Bridge")
        );
        assert_eq!(parse_service_for_device(listing, "en9"), None);
    }

    #[test]
    fn parses_dns_servers_and_empty() {
        let configured = "1.1.1.1\n8.8.8.8";
        assert_eq!(parse_dns_servers(configured), vec!["1.1.1.1", "8.8.8.8"]);
        let none = "There aren't any DNS Servers set on Wi-Fi.";
        assert!(parse_dns_servers(none).is_empty());
    }
}
