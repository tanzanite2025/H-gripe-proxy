//! In-memory smoltcp [`Device`] backed by two frame queues, plus the transparent
//! [`Interface`] the poll loop drives.

use std::collections::VecDeque;
use std::time::Instant as StdInstant;

use smoltcp::iface::{Config as IfaceConfig, Interface};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant as SmolInstant;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address, Ipv6Address};

/// Build the interface in transparent mode: `any_ip` lets it accept packets
/// destined to addresses it does not own, and the catch-all assigned addresses
/// plus default routes let it source replies from the destination the client
/// actually targeted.
pub(super) fn build_interface(phy: &mut TunPhy, now: SmolInstant) -> Interface {
    let config = IfaceConfig::new(HardwareAddress::Ip);
    let mut iface = Interface::new(config, phy, now);
    iface.set_any_ip(true);
    iface.update_ip_addrs(|addrs| {
        let _ = addrs.push(IpCidr::new(IpAddress::Ipv4(Ipv4Address::new(0, 0, 0, 1)), 0));
        let _ = addrs.push(IpCidr::new(
            IpAddress::Ipv6(Ipv6Address::new(0, 0, 0, 0, 0, 0, 0, 1)),
            0,
        ));
    });
    let _ = iface.routes_mut().add_default_ipv4_route(Ipv4Address::new(0, 0, 0, 1));
    let _ = iface
        .routes_mut()
        .add_default_ipv6_route(Ipv6Address::new(0, 0, 0, 0, 0, 0, 0, 1));
    iface
}

pub(super) fn smol_now(start: StdInstant) -> SmolInstant {
    SmolInstant::from_micros(start.elapsed().as_micros() as i64)
}

/// In-memory smoltcp [`Device`] backed by two frame queues the poll loop fills
/// and drains.
pub(super) struct TunPhy {
    pub(super) rx: VecDeque<Vec<u8>>,
    pub(super) tx: VecDeque<Vec<u8>>,
    mtu: usize,
}

impl TunPhy {
    pub(super) fn new(mtu: usize) -> Self {
        Self {
            rx: VecDeque::new(),
            tx: VecDeque::new(),
            mtu,
        }
    }
}

pub(super) struct PhyRxToken {
    buf: Vec<u8>,
}

pub(super) struct PhyTxToken<'a> {
    tx: &'a mut VecDeque<Vec<u8>>,
}

impl Device for TunPhy {
    type RxToken<'a> = PhyRxToken;
    type TxToken<'a> = PhyTxToken<'a>;

    fn receive(&mut self, _timestamp: SmolInstant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let buf = self.rx.pop_front()?;
        Some((PhyRxToken { buf }, PhyTxToken { tx: &mut self.tx }))
    }

    fn transmit(&mut self, _timestamp: SmolInstant) -> Option<Self::TxToken<'_>> {
        Some(PhyTxToken { tx: &mut self.tx })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = self.mtu;
        caps
    }
}

impl RxToken for PhyRxToken {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.buf)
    }
}

impl TxToken for PhyTxToken<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = vec![0u8; len];
        let result = f(&mut buf);
        self.tx.push_back(buf);
        result
    }
}
