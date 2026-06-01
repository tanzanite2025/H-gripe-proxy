pub mod decoy_file;
pub mod memory;
pub mod secure_storage;

pub use decoy_file::ConfigDecoy;
pub use memory::{
    check_global_honeypot, detect_memory_scanning, get_global_honeypot_stats,
    init_global_honeypot, init_global_honeypot_with_count, monitor_loop, HoneypotStats,
};
pub use secure_storage::{generate_encryption_key, SecureConfigStorage};
