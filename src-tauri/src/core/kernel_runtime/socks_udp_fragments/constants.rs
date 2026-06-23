pub(super) const RUST_SOCKS_UDP_FRAGMENTS_COMPONENT: &str = "rust-socks-udp-fragments-execution";
pub(super) const RUST_SOCKS_UDP_FRAGMENTS_KERNEL_AREA: &str = "socks-udp-fragments";
pub(super) const RUST_SOCKS_UDP_FRAGMENTS_EVIDENCE_FILE: &str = "evidence.yaml";
pub(super) const RUST_SOCKS_UDP_FRAGMENTS_ROLLBACK_FILE: &str = "rollback-checkpoint.yaml";
pub(super) const RUST_SOCKS_UDP_FRAGMENTS_OWNED_SCOPE: &str =
    "SOCKS5 UDP two-fragment loopback reassembly and forwarding";
pub(super) const NEXT_SAFE_BATCH: &str = "unsupported-protocol-and-packet-capture-implementation";
pub(super) const SOCKS_UDP_ECHO_PREFIX: &[u8] = b"udp-fragments-ok:";
pub(super) const SOCKS_UDP_FRAGMENT_ONE: u8 = 0x01;
pub(super) const SOCKS_UDP_FRAGMENT_FINAL_TWO: u8 = 0x82;
pub(super) const SOCKS_UDP_FINAL_MASK: u8 = 0x80;
pub(super) const SOCKS_UDP_FRAGMENT_INDEX_MASK: u8 = 0x7f;
pub(super) const TEST_FRAGMENT_ONE: &[u8] = b"bounded socks ";
pub(super) const TEST_FRAGMENT_TWO: &[u8] = b"udp fragments payload";
