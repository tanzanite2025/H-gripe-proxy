//! In-process core-log streaming.
//!
//! Replaces the former Mihomo controller `/logs` WebSocket. The Rust kernel
//! (`learn-gripe`) is now the proxy core and emits its logs through the global
//! [`log`] facade, which the app's `flexi_logger` already captures. Rather than
//! querying an external controller, we tap that pipeline: a [`LogLineFilter`]
//! wrapper ([`CoreLogTap`]) forwards every kernel-originated record to an
//! in-process broadcast channel, and consumers (the log monitor) subscribe to
//! it and push frontend-shaped JSON to the UI.
//!
//! Only records emitted by the kernel crate (`learn_gripe` module path) are
//! broadcast, matching the old behavior where `/logs` carried the proxy core's
//! logs and not the app's own logging.

use clash_dtos::LogLevel;
use flexi_logger::DeferredNow;
use flexi_logger::filter::{LogLineFilter, LogLineWriter};
use log::Record;
use once_cell::sync::Lazy;
use serde_json::{Value, json};
use tokio::sync::broadcast;

/// Bound on buffered core-log records. A slow consumer simply drops the oldest
/// records (`RecvError::Lagged`) rather than blocking the logging path.
const CORE_LOG_CHANNEL_CAPACITY: usize = 512;

/// Module-path prefix identifying kernel-originated log records.
const KERNEL_MODULE: &str = "learn_gripe";

/// A single core-log record captured from the kernel's logging.
#[derive(Clone, Debug)]
pub struct CoreLogRecord {
    pub level: log::Level,
    pub message: String,
}

static CORE_LOG_TX: Lazy<broadcast::Sender<CoreLogRecord>> = Lazy::new(|| {
    let (tx, _rx) = broadcast::channel(CORE_LOG_CHANNEL_CAPACITY);
    tx
});

/// Subscribe to the in-process core-log broadcast.
pub fn subscribe() -> broadcast::Receiver<CoreLogRecord> {
    CORE_LOG_TX.subscribe()
}

/// Publish a core-log record to all current subscribers. Cheap and
/// non-blocking; ignores the "no subscribers" case.
pub fn publish(record: CoreLogRecord) {
    let _ = CORE_LOG_TX.send(record);
}

fn is_kernel_record(record: &Record) -> bool {
    record
        .module_path()
        .map(|module| module == KERNEL_MODULE || module.starts_with("learn_gripe::"))
        .unwrap_or(false)
}

/// Frontend log type string for a [`log::Level`]. The UI only distinguishes
/// `debug`/`info`/`warning`/`error`, so `Trace` folds into `debug`.
fn level_type(level: log::Level) -> &'static str {
    match level {
        log::Level::Error => "error",
        log::Level::Warn => "warning",
        log::Level::Info => "info",
        log::Level::Debug | log::Level::Trace => "debug",
    }
}

/// Render a record as the Mihomo-compatible `{type, payload}` JSON the frontend
/// log panel expects. The frontend stamps its own receive time, so `time` is
/// intentionally omitted.
pub fn to_frontend_value(record: &CoreLogRecord) -> Value {
    json!({
        "type": level_type(record.level),
        "payload": record.message,
    })
}

fn severity(level: log::Level) -> u8 {
    match level {
        log::Level::Error => 4,
        log::Level::Warn => 3,
        log::Level::Info => 2,
        log::Level::Debug => 1,
        log::Level::Trace => 0,
    }
}

/// Whether a record at `record_level` should be delivered to a subscriber that
/// requested `requested` as its minimum level (matching Mihomo `/logs?level=`).
pub fn level_passes(requested: LogLevel, record_level: log::Level) -> bool {
    let min = match requested {
        LogLevel::SILENT => return false,
        LogLevel::ERROR => 4,
        LogLevel::WARNING => 3,
        LogLevel::INFO => 2,
        // DEBUG includes both Debug and Trace.
        LogLevel::DEBUG => 0,
    };
    severity(record_level) >= min
}

/// A [`LogLineFilter`] that taps kernel-originated records into the core-log
/// broadcast before delegating to an inner filter for the actual file/stdout
/// write. Composing rather than replacing keeps the existing logging behavior
/// (rotation, module filtering, formatting) unchanged.
pub struct CoreLogTap {
    inner: Box<dyn LogLineFilter + Send + Sync>,
}

impl CoreLogTap {
    pub fn new(inner: Box<dyn LogLineFilter + Send + Sync>) -> Self {
        Self { inner }
    }
}

impl LogLineFilter for CoreLogTap {
    fn write(
        &self,
        now: &mut DeferredNow,
        record: &Record,
        log_line_writer: &dyn LogLineWriter,
    ) -> std::io::Result<()> {
        if is_kernel_record(record) {
            publish(CoreLogRecord {
                level: record.level(),
                message: record.args().to_string(),
            });
        }
        self.inner.write(now, record, log_line_writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{Level, RecordBuilder};

    #[test]
    fn level_type_maps_trace_to_debug() {
        assert_eq!(level_type(Level::Error), "error");
        assert_eq!(level_type(Level::Warn), "warning");
        assert_eq!(level_type(Level::Info), "info");
        assert_eq!(level_type(Level::Debug), "debug");
        assert_eq!(level_type(Level::Trace), "debug");
    }

    #[test]
    fn level_passes_respects_threshold() {
        // SILENT drops everything.
        assert!(!level_passes(LogLevel::SILENT, Level::Error));

        // ERROR only keeps errors.
        assert!(level_passes(LogLevel::ERROR, Level::Error));
        assert!(!level_passes(LogLevel::ERROR, Level::Warn));

        // WARNING keeps warn and above.
        assert!(level_passes(LogLevel::WARNING, Level::Warn));
        assert!(!level_passes(LogLevel::WARNING, Level::Info));

        // INFO keeps info and above.
        assert!(level_passes(LogLevel::INFO, Level::Info));
        assert!(!level_passes(LogLevel::INFO, Level::Debug));

        // DEBUG keeps everything, including trace.
        assert!(level_passes(LogLevel::DEBUG, Level::Debug));
        assert!(level_passes(LogLevel::DEBUG, Level::Trace));
    }

    #[test]
    fn frontend_value_shape() {
        let record = CoreLogRecord {
            level: Level::Warn,
            message: "learn-gripe accept error: boom".to_string(),
        };
        let value = to_frontend_value(&record);
        assert_eq!(value["type"], "warning");
        assert_eq!(value["payload"], "learn-gripe accept error: boom");
        assert!(value.get("time").is_none());
    }

    #[test]
    fn kernel_record_detection() {
        let kernel = RecordBuilder::new()
            .level(Level::Info)
            .module_path(Some("learn_gripe::inbound"))
            .args(format_args!("listening"))
            .build();
        assert!(is_kernel_record(&kernel));

        let kernel_root = RecordBuilder::new()
            .level(Level::Info)
            .module_path(Some("learn_gripe"))
            .args(format_args!("root"))
            .build();
        assert!(is_kernel_record(&kernel_root));

        let app = RecordBuilder::new()
            .level(Level::Info)
            .module_path(Some("clash_verge_optimized::core"))
            .args(format_args!("app log"))
            .build();
        assert!(!is_kernel_record(&app));

        let lookalike = RecordBuilder::new()
            .level(Level::Info)
            .module_path(Some("learn_gripe_extra::thing"))
            .args(format_args!("not the kernel"))
            .build();
        assert!(!is_kernel_record(&lookalike));
    }

    #[tokio::test]
    async fn publish_reaches_subscriber() {
        let mut rx = subscribe();
        publish(CoreLogRecord {
            level: Level::Info,
            message: "hello".to_string(),
        });
        let got = rx.recv().await.expect("record");
        assert_eq!(got.level, Level::Info);
        assert_eq!(got.message, "hello");
    }
}
