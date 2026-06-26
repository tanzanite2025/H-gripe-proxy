mod config;
mod lifecycle;
mod outbound_select;
mod state;
mod tun_inbound;

use crate::singleton;
use anyhow::Result;
use arc_swap::{ArcSwap, ArcSwapOption};
use clash_verge_logging::AsyncLogger;
use once_cell::sync::Lazy;
use std::{fmt, sync::Arc, time::Instant};

pub(crate) static CLASH_LOGGER: Lazy<Arc<AsyncLogger>> = Lazy::new(|| Arc::new(AsyncLogger::new()));

#[derive(Debug, serde::Serialize, PartialEq, Eq)]
pub enum RunningMode {
    Service,
    NotRunning,
    /// The Rust-owned learn-gripe data plane is running in-process.
    Gripe,
}

impl fmt::Display for RunningMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Service => write!(f, "Service"),
            Self::NotRunning => write!(f, "NotRunning"),
            Self::Gripe => write!(f, "Gripe"),
        }
    }
}

#[derive(Debug)]
pub struct CoreManager {
    state: ArcSwap<State>,
    last_update: ArcSwapOption<Instant>,
    gripe: tokio::sync::Mutex<Option<learn_gripe::GripeHandle>>,
    /// The OS TUN inbound, present only while TUN mode is enabled and running.
    tun: tokio::sync::Mutex<Option<tun_inbound::TunInbound>>,
}

#[derive(Debug)]
struct State {
    running_mode: ArcSwap<RunningMode>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            running_mode: ArcSwap::new(Arc::new(RunningMode::NotRunning)),
        }
    }
}

impl Default for CoreManager {
    fn default() -> Self {
        Self {
            state: ArcSwap::new(Arc::new(State::default())),
            last_update: ArcSwapOption::new(None),
            gripe: tokio::sync::Mutex::new(None),
            tun: tokio::sync::Mutex::new(None),
        }
    }
}

impl CoreManager {
    fn new() -> Self {
        Self::default()
    }

    pub fn get_running_mode(&self) -> Arc<RunningMode> {
        Arc::clone(&self.state.load().running_mode.load())
    }

    pub fn get_last_update(&self) -> Option<Arc<Instant>> {
        self.last_update.load_full()
    }

    pub fn set_running_mode(&self, mode: RunningMode) {
        let state = self.state.load();
        state.running_mode.store(Arc::new(mode));
    }

    pub fn set_last_update(&self, time: Instant) {
        self.last_update.store(Some(Arc::new(time)));
    }

    pub async fn init(&self) -> Result<()> {
        self.start_core().await?;
        Ok(())
    }
}

singleton!(CoreManager, CORE_MANAGER);
