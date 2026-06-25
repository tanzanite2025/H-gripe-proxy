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
//! it also installs a **global default-route capture** so all system traffic is
//! pulled into the TUN — but *only* when the selected outbound is a single,
//! fixed-server proxy (see [`OutboundMode::supports_global_capture`]): the proxy
//! server's own IP is pinned to the physical gateway with a `/32` bypass route
//! (so it is not looped back into the tunnel), two `0.0.0.0/1` + `128.0.0.0/1`
//! routes (more specific than the untouched `0.0.0.0/0` default) point the rest
//! at the TUN, and the resolver is pointed at the in-stack fake-IP DNS. After
//! applying, the route table is re-read to confirm the capture took effect; if
//! not, everything is rolled back and start fails. `Direct`/`Reject`/`Routed`
//! outbounds cannot be globally captured without looping arbitrary or
//! Direct-routed targets, so they fall back to serving only the on-link subnet.
//!
//! **Untested.** The capture shells out to `route`/`netsh` and needs admin plus
//! a real default route; it is compile-verified only and **must** be validated
//! on a real Windows machine. IPv6 is not captured (a known leak gap).
//!
//! Every privileged system mutation is pushed onto a [`RollbackStack`] with its
//! inverse and undone in reverse order on [`TunInbound::stop`] (and on `Drop` as
//! a safety net), so enabling TUN never leaves the OS in a half-configured
//! state. This whole path is gated behind `enable_tun_mode` and is off by
//! default; it has been compile-verified but must be validated on a real machine
//! with administrator/root privileges.

use anyhow::{Context, Result, anyhow, bail};
use clash_verge_logging::{Type, logging};
use learn_gripe::{DEFAULT_MTU, DnsMode, FakeIpConfig, OutboundMode, serve_tun_device};
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tun::AbstractDevice;

/// Address assigned to the TUN interface. 198.18.0.0/15 is the IANA
/// benchmarking range — unlikely to collide with real networks.
const TUN_ADDRESS: Ipv4Addr = Ipv4Addr::new(198, 18, 0, 1);
const TUN_NETMASK: Ipv4Addr = Ipv4Addr::new(255, 255, 0, 0);
const TUN_NAME: &str = "clash-verge";

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
            .tun_name(TUN_NAME)
            .address(TUN_ADDRESS)
            .netmask(TUN_NETMASK)
            .mtu(DEFAULT_MTU as u16)
            .up();
        // Deliver raw L3 frames with no packet-information header, matching the
        // contract `serve_tun_device` expects (Windows wintun has none).
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios"))]
        config.platform_config(|p| {
            p.packet_information(false);
        });

        let device = tun::create_as_async(&config).map_err(|err| anyhow!("failed to create TUN device: {err}"))?;
        // Read the interface index before splitting consumes the device; the
        // global-capture routes pin themselves to this interface.
        let if_index = device
            .tun_index()
            .map_err(|err| anyhow!("failed to read TUN interface index: {err}"))?;
        let (reader, writer) = tokio::io::split(device);

        // Answer DNS over the TUN from a fake-IP pool on the interface subnet so
        // a global capture won't black-hole name resolution. Reserve the gateway
        // (the interface address) so it is never handed out as a fake IP.
        let (dns, pool) = DnsMode::fake_ip(FakeIpConfig::default());
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

        // Global default-route capture (Windows). Sound only for a single,
        // fixed-server proxy outbound; on failure `rollback` (dropped on this
        // error path) undoes any partially-applied routes.
        #[cfg(windows)]
        if outbound.supports_global_capture() {
            if let Err(err) = install_global_capture(&mut rollback, if_index, &outbound) {
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
        #[cfg(not(windows))]
        {
            let _ = if_index;
            logging!(
                warn,
                Type::Core,
                "TUN global capture is only implemented on Windows; serving on-link subnet only"
            );
        }

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
        })
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

#[cfg(windows)]
fn str_refs(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

/// Run a system command and return its stdout, erroring on non-zero exit.
#[cfg(windows)]
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

/// Install the Windows global default-route capture, recording each mutation's
/// inverse on `rollback`. See the module docs for the design; this shells out to
/// `route`/`netsh` and is compile-verified only.
#[cfg(windows)]
fn install_global_capture(rollback: &mut RollbackStack, if_index: i32, outbound: &OutboundMode) -> Result<()> {
    use std::net::{IpAddr, ToSocketAddrs};

    // 1. Resolve the proxy server endpoint(s) to literal IPv4s while normal DNS
    //    still works (before the default route is captured).
    let mut server_ips: Vec<Ipv4Addr> = Vec::new();
    for (host, port) in outbound.direct_dial_endpoints() {
        let resolved = (host.as_str(), port)
            .to_socket_addrs()
            .with_context(|| format!("resolve proxy server {host}:{port} for the bypass route"))?;
        for addr in resolved {
            if let IpAddr::V4(ip) = addr.ip() {
                if !server_ips.contains(&ip) {
                    server_ips.push(ip);
                }
            }
        }
    }
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
}
