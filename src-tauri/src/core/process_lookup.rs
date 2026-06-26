//! Bridges the kernel's `PROCESS-NAME` / `PROCESS-PATH` matchers to the
//! operating system.
//!
//! The `learn-gripe` router only knows how to *query* the local process that
//! owns a connection's source socket via the [`learn_gripe::ProcessLookup`]
//! trait; it never performs the OS-level lookup itself. [`ProcessData`] is the
//! embedder-side implementation of that trait. It reuses the cross-platform
//! port→process resolution the app already relies on for session affinity
//! (`session_affinity::process_detection`), mapping a connection's source port
//! to the owning process name and executable path.

use std::net::SocketAddr;

use learn_gripe::{ConnNetwork, ProcessInfo, ProcessLookup};

use super::session_affinity::process_detection;

/// Resolves the local process that owns a connection's source socket, backed by
/// the OS-level port→process lookup the app already uses for session affinity.
///
/// Name and path are resolved independently and best-effort: whichever the OS
/// can supply is filled in, the other is left empty. When neither can be
/// resolved the lookup yields `None`, so `PROCESS-NAME` / `PROCESS-PATH` rules
/// simply do not match — the same "no data ⇒ never match" contract the router
/// uses for `GEOIP` / `RULE-SET` when their data is absent.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessData;

impl ProcessLookup for ProcessData {
    fn lookup(&self, _network: ConnNetwork, src: SocketAddr) -> Option<ProcessInfo> {
        let port = src.port();
        let name = process_detection::get_process_name_by_port(port).unwrap_or_default();
        let path = process_detection::get_process_path_by_port(port).unwrap_or_default();
        if name.is_empty() && path.is_empty() {
            return None;
        }
        Some(ProcessInfo { name, path })
    }
}
