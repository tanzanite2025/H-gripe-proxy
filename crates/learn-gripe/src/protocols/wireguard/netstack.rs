// --- smoltcp in-memory device --------------------------------------------------

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Instant;

use smoltcp::iface::{Config as IfaceConfig, Interface};
use smoltcp::socket::tcp;
use smoltcp::time::Instant as SmolInstant;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr};

/// In-memory smoltcp [`Device`](smoltcp::phy::Device) backed by two frame
/// queues: `tx` holds IP packets the stack wants encrypted + sent to the peer,
/// `rx` holds decrypted IP packets from the peer waiting to enter the stack.
pub(super) struct WgPhy {
    pub(super) rx: std::collections::VecDeque<Vec<u8>>,
    pub(super) tx: std::collections::VecDeque<Vec<u8>>,
    mtu: usize,
}

impl WgPhy {
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

impl smoltcp::phy::Device for WgPhy {
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

/// Build the userspace interface, assigning the peer-given tunnel address(es) at
/// prefix 0 so every inner destination is treated as on-link (the tunnel is the
/// only egress) while replies still source from our assigned address.
pub(super) fn build_interface(
    phy: &mut WgPhy,
    now: SmolInstant,
    local_v4: Option<Ipv4Addr>,
    local_v6: Option<Ipv6Addr>,
) -> Interface {
    let config = IfaceConfig::new(HardwareAddress::Ip);
    let mut iface = Interface::new(config, phy, now);
    iface.set_any_ip(true);
    iface.update_ip_addrs(|addrs| {
        if let Some(v4) = local_v4 {
            let _ = addrs.push(IpCidr::new(IpAddress::Ipv4(v4), 0));
        }
        if let Some(v6) = local_v6 {
            let _ = addrs.push(IpCidr::new(IpAddress::Ipv6(v6), 0));
        }
    });
    if let Some(v4) = local_v4 {
        let _ = iface.routes_mut().add_default_ipv4_route(v4);
    }
    if let Some(v6) = local_v6 {
        let _ = iface.routes_mut().add_default_ipv6_route(v6);
    }
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
