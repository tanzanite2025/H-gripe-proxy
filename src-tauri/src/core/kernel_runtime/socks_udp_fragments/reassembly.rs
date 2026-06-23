use super::protocol::SocksUdpFragment;
use anyhow::{Result, anyhow};
use std::net::SocketAddr;

pub(super) fn reassemble_ordered_fragments(fragments: &[SocksUdpFragment]) -> Result<Vec<u8>> {
    if fragments.len() != 2 {
        return Err(anyhow!("bounded SOCKS UDP fragment path requires two fragments"));
    }
    let target = fragments[0].target;
    let mut payload = Vec::new();
    for (offset, fragment) in fragments.iter().enumerate() {
        if fragment.target != target {
            return Err(anyhow!("SOCKS UDP fragments target different destinations"));
        }
        ensure_loopback_target(fragment.target)?;
        let expected_index = u8::try_from(offset + 1)?;
        if fragment.index() != expected_index {
            return Err(anyhow!("SOCKS UDP fragments arrived out of bounded order"));
        }
        if fragment.is_final() != (offset + 1 == fragments.len()) {
            return Err(anyhow!("SOCKS UDP final-fragment marker is invalid"));
        }
        payload.extend_from_slice(&fragment.payload);
    }
    Ok(payload)
}

pub(super) fn ensure_loopback_target(target: SocketAddr) -> Result<()> {
    if target.ip().is_loopback() {
        Ok(())
    } else {
        Err(anyhow!(
            "SOCKS UDP target {target} is outside the bounded loopback scope"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::kernel_runtime::socks_udp_fragments::{
        constants::{SOCKS_UDP_FRAGMENT_FINAL_TWO, SOCKS_UDP_FRAGMENT_ONE},
        protocol::{encode_socks_udp_fragment, parse_socks_udp_fragment},
    };
    use std::net::Ipv4Addr;

    #[test]
    fn reassembles_two_loopback_fragments() {
        let target = SocketAddr::from((Ipv4Addr::LOCALHOST, 2082));
        let fragments = [
            parse_socks_udp_fragment(&encode_socks_udp_fragment(target, SOCKS_UDP_FRAGMENT_ONE, b"one-")).unwrap(),
            parse_socks_udp_fragment(&encode_socks_udp_fragment(target, SOCKS_UDP_FRAGMENT_FINAL_TWO, b"two")).unwrap(),
        ];

        assert_eq!(reassemble_ordered_fragments(&fragments).unwrap(), b"one-two");
    }
}
