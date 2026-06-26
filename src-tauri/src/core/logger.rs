use std::{
    str::FromStr as _,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

use anyhow::{Result, bail};
use clash_verge_logging::{Type, logging};
use flexi_logger::{
    Cleanup, Criterion, FileSpec, LogSpecBuilder, LogSpecification, LoggerHandle,
    writers::{FileLogWriter, FileLogWriterBuilder},
};
use log::LevelFilter;
use parking_lot::{Mutex, RwLock};

use crate::{singleton, utils::dirs};

pub struct Logger {
    handle: Arc<Mutex<Option<LoggerHandle>>>,
    log_level: Arc<RwLock<LevelFilter>>,
    log_max_size: AtomicU64,
    log_max_count: AtomicUsize,
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            handle: Arc::new(Mutex::new(None)),
            log_level: Arc::new(RwLock::new(LevelFilter::Info)),
            log_max_size: AtomicU64::new(128),
            log_max_count: AtomicUsize::new(8),
        }
    }
}

singleton!(Logger, LOGGER);

impl Logger {
    fn new() -> Self {
        Self::default()
    }

    pub async fn init(&self) -> Result<()> {
        let (log_level, log_max_size, log_max_count) = {
            let verge_guard = crate::config::Config::verge().await;
            let verge = verge_guard.latest_arc();
            (
                verge.get_log_level(),
                verge.app_log_max_size.unwrap_or(128),
                verge.app_log_max_count.unwrap_or(8),
            )
        };
        let log_level = std::env::var("RUST_LOG")
            .ok()
            .and_then(|v| log::LevelFilter::from_str(&v).ok())
            .unwrap_or(log_level);
        *self.log_level.write() = log_level;
        self.log_max_size.store(log_max_size, Ordering::SeqCst);
        self.log_max_count.store(log_max_count, Ordering::SeqCst);

        #[cfg(not(any(feature = "tauri-dev", feature = "tokio-trace")))]
        {
            let log_spec = Self::generate_log_spec(log_level);
            let log_dir = dirs::app_logs_dir()?;
            let logger = flexi_logger::Logger::with(log_spec)
                .log_to_file(FileSpec::default().directory(log_dir).basename(""))
                .duplicate_to_stdout(log_level.into())
                .format(clash_verge_logger::console_format)
                .format_for_files(clash_verge_logger::file_format_with_level)
                .rotate(
                    Criterion::Size(log_max_size * 1024),
                    flexi_logger::Naming::TimestampsCustomFormat {
                        current_infix: Some("latest"),
                        format: "%Y-%m-%d_%H-%M-%S",
                    },
                    Cleanup::KeepLogFiles(log_max_count),
                );

            let mut filter_modules = vec!["wry", "tokio_tungstenite", "tungstenite"];
            #[cfg(not(feature = "tracing"))]
            filter_modules.push("tauri");
            #[cfg(feature = "tracing")]
            filter_modules.push("kode_bridge");
            let logger = logger.filter(Box::new(crate::core::log_stream::CoreLogTap::new(Box::new(
                clash_verge_logging::NoModuleFilter(filter_modules),
            ))));

            let handle = logger.start()?;
            *self.handle.lock() = Some(handle);
        }

        std::panic::set_hook(Box::new(move |info| {
            let payload = info
                .payload()
                .downcast_ref::<&str>()
                .unwrap_or(&"Unknown panic payload");
            let location = info
                .location()
                .map(|loc| format!("{}:{}", loc.file(), loc.line()))
                .unwrap_or_else(|| "Unknown location".to_string());
            logging!(error, Type::System, "Panic occurred at {}: {}", location, payload);
            if let Some(h) = Self::global().handle.lock().as_ref() {
                h.flush();
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }));

        Ok(())
    }

    fn generate_log_spec(log_level: LevelFilter) -> LogSpecification {
        let mut spec = LogSpecBuilder::new();
        let log_level = std::env::var("RUST_LOG")
            .ok()
            .and_then(|v| log::LevelFilter::from_str(&v).ok())
            .unwrap_or(log_level);
        spec.default(log_level);
        #[cfg(feature = "tracing")]
        spec.module("tauri", log::LevelFilter::Debug)
            .module("wry", log::LevelFilter::Off);
        spec.build()
    }

    fn generate_file_log_writer(&self) -> Result<FileLogWriterBuilder> {
        let log_dir = dirs::app_logs_dir()?;
        let log_max_size = self.log_max_size.load(Ordering::SeqCst);
        let log_max_count = self.log_max_count.load(Ordering::SeqCst);
        let flwb = FileLogWriter::builder(FileSpec::default().directory(log_dir).basename("")).rotate(
            Criterion::Size(log_max_size * 1024),
            flexi_logger::Naming::TimestampsCustomFormat {
                current_infix: Some("latest"),
                format: "%Y-%m-%d_%H-%M-%S",
            },
            Cleanup::KeepLogFiles(log_max_count),
        );
        Ok(flwb)
    }

    /// only update app log level
    pub fn update_log_level(&self, level: LevelFilter) -> Result<()> {
        *self.log_level.write() = level;
        let log_level = self.log_level.read().to_owned();
        if let Some(handle) = self.handle.lock().as_mut() {
            let log_spec = Self::generate_log_spec(log_level);
            handle.set_new_spec(log_spec);
            handle.adapt_duplication_to_stdout(log_level.into())?;
        } else {
            bail!("failed to get logger handle, make sure it init");
        };
        Ok(())
    }

    /// update app and mihomo core log config
    pub async fn update_log_config(&self, log_max_size: u64, log_max_count: usize) -> Result<()> {
        self.log_max_size.store(log_max_size, Ordering::SeqCst);
        self.log_max_count.store(log_max_count, Ordering::SeqCst);
        if let Some(handle) = self.handle.lock().as_ref() {
            let log_file_writer = self.generate_file_log_writer()?;
            handle.reset_flw(&log_file_writer)?;
        } else {
            bail!("failed to get logger handle, make sure it init");
        };
        Ok(())
    }
}
