mod constants;
mod helpers;
pub mod geoip;
mod dns_leak;
mod proxy_detection;
mod runtime_state;

pub use dns_leak::build_dns_leak_test_result;
pub use proxy_detection::build_proxy_detection_result;
pub use runtime_state::build_dns_runtime_status;