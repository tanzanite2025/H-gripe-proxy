//! OS TUN device binding for the learn-gripe TUN inbound.
//!
//! This creates a real OS TUN interface (wintun on Windows, `/dev/net/tun` on
//! Linux, utun on macOS), brings it up with an address, and pumps its IP frames
//! through [`learn_gripe::serve_tun_device`] — relaying each TCP flow through
//! the selected outbound.
//!
//! **Scope / safety.** This binds the device and relays **TCP** only. It does
//! *not* install a global default route to capture all system traffic: with
//! UDP-over-TUN (hence DNS-over-TUN) still unimplemented, a global capture would
//! black-hole UDP/DNS and break name resolution even with a perfect rollback.
//! Global capture must therefore land together with UDP/DNS-over-TUN. For now
//! only the interface subnet (assigned by the OS when the address is brought up)
//! is reachable, so traffic must be explicitly directed at the TUN address range
//! to exercise the path.
//!
//! Every privileged system mutation is pushed onto a [`RollbackStack`] with its
//! inverse and undone in reverse order on [`TunInbound::stop`] (and on `Drop` as
//! a safety net), so enabling TUN never leaves the OS in a half-configured
//! state. This whole path is gated behind `enable_tun_mode` and is off by
//! default; it has been compile-verified but must be validated on a real machine
//! with administrator/root privileges.

use anyhow::{Result, anyhow};
use clash_verge_logging::{Type, logging};
use learn_gripe::{DEFAULT_MTU, OutboundMode, serve_tun_device};
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

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
            "starting experimental learn-gripe TUN inbound on {} ({}/16, mtu {}); TCP-only, no global route capture",
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
        let (reader, writer) = tokio::io::split(device);

        let shutdown = Arc::new(Notify::new());
        let mut rollback = RollbackStack::new();
        // The OS removes the interface (and its auto-added subnet route) when the
        // device is dropped, which happens when the pump task returns after
        // `shutdown`. Record that teardown explicitly so the intent is visible.
        let teardown_shutdown = shutdown.clone();
        rollback.push("bring down and remove the TUN interface", move || {
            teardown_shutdown.notify_waiters();
        });

        let pump = tokio::spawn(serve_tun_device(
            reader,
            writer,
            outbound,
            None,
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
