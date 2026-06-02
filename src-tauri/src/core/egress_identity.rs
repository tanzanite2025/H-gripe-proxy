pub mod config;
pub mod manager;

#[allow(unused_imports)]
pub use config::{
    DnsMode, DnsPolicy, EgressFailoverPolicy, EgressIdentityConfig, EgressIdentityProfile, IdentitySessionPolicy,
};

pub use manager::{EgressIdentityManager, EgressNodeMetadata, EgressSelectionContext, ResolvedEgressIdentity};

#[cfg(test)]
mod tests;
