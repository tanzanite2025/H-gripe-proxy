//! In-memory smoltcp device for the OpenVPN userspace netstack.
//!
//! Mirrors the WireGuard netstack: inner IP packets that smoltcp wants sent go
//! into `tx` (encrypted into `P_DATA_V2` by the device loop), decrypted inner
//! packets from the server go into `rx`.

use std::net::{IpAddr, Ipv4Addr};
use std::time::Instant;

use smoltcp::iface::{Config as IfaceConfig, Interface};
use smoltcp::socket::tcp;
use smoltcp::time::Instant as SmolInstant;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr};

pub(super) struct OvPhy {
    pub(super) rx: std::collections::VecDeque<Vec<u8>>,
    pub(super) tx: std::collections::VecDeque<Vec<u8>>,
    mtu: usize,
}

impl OvPhy {
    pub(super) fn new(mtu: usize) -> Self {
        Self {
            rx: std::collections::VecDeque::new(),
            tx: std::collections::VecDeque::new(),
            mtu,
        }
    }
}

pub(super) struct PhyRxToken {
    buf: Vec<u8>,
}

pub(super) struct PhyTxToken<'a> {
    tx: &'a mut std::collections::VecDeque<Vec<u8>>,
}

impl smoltcp::phy::Device for OvPhy {
    type RxToken<'a> = PhyRxToken;
    type TxToken<'a> = PhyTxToken<'a>;

    fn receive(&mut self, _t: SmolInstant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let buf = self.rx.pop_front()?;
        Some((PhyRxToken { buf }, PhyTxToken { tx: &mut self.tx }))
    }

    fn transmit(&mut self, _t: SmolInstant) -> Option<Self::TxToken<'_>> {
        Some(PhyTxToken { tx: &mut self.tx })
    }

    fn capabilities(&self) -> smoltcp::phy::DeviceCapabilities {
        let mut caps = smoltcp::phy::DeviceCapabilities::default();
        caps.medium = smoltcp::phy::Medium::Ip;
        caps.max_transmission_unit = self.mtu;
        caps
    }
}

impl smoltcp::phy::RxToken for PhyRxToken {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.buf)
    }
}

impl smoltcp::phy::TxToken for PhyTxToken<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = vec![0u8; len];
        let result = f(&mut buf);
        self.tx.push_back(buf);
        result
    }
}

/// Build the userspace interface, assigning the pushed tunnel address at prefix
/// 0 so every inner destination is on-link (the tunnel is the only egress).
pub(super) fn build_interface(phy: &mut OvPhy, now: SmolInstant, local_v4: Ipv4Addr) -> Interface {
    let config = IfaceConfig::new(HardwareAddress::Ip);
    let mut iface = Interface::new(config, phy, now);
    iface.set_any_ip(true);
    iface.update_ip_addrs(|addrs| {
        let _ = addrs.push(IpCidr::new(IpAddress::Ipv4(local_v4), 0));
    });
    let _ = iface.routes_mut().add_default_ipv4_route(local_v4);
    iface
}

pub(super) fn smol_now(start: Instant) -> SmolInstant {
    SmolInstant::from_micros(start.elapsed().as_micros() as i64)
}

pub(super) fn ip_address(ip: IpAddr) -> IpAddress {
    match ip {
        IpAddr::V4(v4) => IpAddress::Ipv4(v4),
        IpAddr::V6(v6) => IpAddress::Ipv6(v6),
    }
}

pub(super) fn is_dead(state: tcp::State) -> bool {
    matches!(state, tcp::State::Closed | tcp::State::TimeWait | tcp::State::Closing)
}
