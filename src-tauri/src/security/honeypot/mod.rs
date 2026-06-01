pub mod decoy_file;
pub mod memory;
pub mod secure_storage;
pub mod strategy;

pub use decoy_file::ConfigDecoy;
pub use memory::{
    check_global_honeypot, detect_memory_scanning, get_global_honeypot_stats,
    init_global_honeypot, init_global_honeypot_with_count, monitor_loop, HoneypotStats,
};
pub use secure_storage::{generate_encryption_key, SecureConfigStorage};
pub use strategy::{
    check_decoy_plan_access, cleanup_decoy_plan, deploy_decoy_plan, DecoyBatchResult,
    DecoyDeploymentPlan,
};
