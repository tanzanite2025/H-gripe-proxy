pub mod config;
pub mod manager;

pub use config::{
    DnsMode,
    EgressIdentityConfig,
};

pub use manager::{
    EgressIdentityManager,
    EgressNodeMetadata,
    EgressSelectionContext,
    ResolvedEgressIdentity,
};

#[cfg(test)]
mod tests;

