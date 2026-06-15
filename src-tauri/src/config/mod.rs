pub mod advanced;
mod clash;
#[allow(clippy::module_inception)]
mod config;
pub mod dns_defaults;
mod encrypt;
mod prfitem;
pub mod profiles;
pub mod runtime;
pub mod traits;
mod verge;

pub use self::{
    advanced::*, clash::*, config::*, dns_defaults::*, encrypt::*, prfitem::*, profiles::*, traits::*, verge::*,
};

pub const DEFAULT_PAC: &str = r#"function FindProxyForURL(url, host) {
  return "PROXY 127.0.0.1:%mixed-port%; SOCKS5 127.0.0.1:%mixed-port%; DIRECT;";
}
"#;
