//! Get/Set system proxy for Windows.

mod windows;

// #[cfg(feature = "utils")]
pub mod utils;

#[cfg(feature = "guard")]
pub mod guard;

#[cfg(feature = "guard")]
pub use guard::{GuardMonitor, GuardType};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Sysproxy {
    pub host: String,
    pub bypass: String,
    pub port: u16,
    pub enable: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Autoproxy {
    pub url: String,
    pub enable: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to parse string `{0}`")]
    ParseStr(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("failed to get default network interface")]
    NetworkInterface,

    #[error("failed to set proxy for this environment")]
    NotSupport,

    #[error("admin privileges required to modify system proxy")]
    RequiresAdminPrivileges,

    #[error("system call failed")]
    SystemCall(#[from] windows::Win32Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Sysproxy {
    pub const fn is_support() -> bool {
        cfg!(target_os = "windows")
    }
}

impl Autoproxy {
    pub const fn is_support() -> bool {
        cfg!(target_os = "windows")
    }
}
