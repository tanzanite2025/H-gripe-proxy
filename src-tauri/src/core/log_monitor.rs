use crate::core::handle;
use crate::process::AsyncHandler;
use crate::utils::connections_stream;
use crate::{Type, logging};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tauri_plugin_mihomo::models::{ConnectionId, LogLevel};

const LOG_MONITOR_RETRY_DELAY: Duration = Duration::from_secs(2);
const LOG_MONITOR_IDLE_INTERVAL: Duration = Duration::from_millis(500);

pub struct LogMonitorController {
    task: Arc<Mutex<Option<JoinHandle<()>>>>,
    connection_id: Arc<Mutex<Option<ConnectionId>>>,
    level: Arc<Mutex<Option<LogLevel>>>,
}

impl Default for LogMonitorController {
    fn default() -> Self {
        Self {
            task: Arc::new(Mutex::new(None)),
            connection_id: Arc::new(Mutex::new(None)),
            level: Arc::new(Mutex::new(None)),
        }
    }
}

impl LogMonitorController {
    pub fn start(&self, level: LogLevel) {
        if handle::Handle::global().is_exiting() {
            return;
        }

        if self.is_running() && self.level.lock().as_ref() == Some(&level) {
            return;
        }

        self.stop();
        *self.level.lock() = Some(level);

        let conn_id = Arc::clone(&self.connection_id);
        let task = AsyncHandler::spawn(move || async move {
            loop {
                if handle::Handle::global().is_exiting() {
                    break;
                }

                match crate::core::runtime_bridge::connect_runtime_log_stream(level, |payload| {
                    handle::Handle::send_core_log(payload);
                })
                .await
                {
                    Ok(id) => {
                        *conn_id.lock() = Some(id);
                        loop {
                            if handle::Handle::global().is_exiting() {
                                break;
                            }
                            tokio::time::sleep(LOG_MONITOR_IDLE_INTERVAL).await;
                        }
                    }
                    Err(err) => {
                        logging!(debug, Type::Core, "Log monitor stream connect failed, retrying: {err}");
                        tokio::time::sleep(LOG_MONITOR_RETRY_DELAY).await;
                    }
                }
            }

            *conn_id.lock() = None;
        });

        *self.task.lock() = Some(task);
    }

    pub fn stop(&self) {
        *self.level.lock() = None;
        let task = self.task.lock().take();
        let connection_id = self.connection_id.lock().take();

        AsyncHandler::spawn(move || async move {
            if let Some(task) = task {
                task.abort();
                let _ = task.await;
            }
            disconnect(connection_id).await;
        });
    }

    fn is_running(&self) -> bool {
        self.task.lock().as_ref().is_some_and(|t| !t.inner().is_finished())
    }
}

async fn disconnect(connection_id: Option<ConnectionId>) {
    if let Some(id) = connection_id {
        connections_stream::disconnect_connection(id).await;
    }
}

static LOG_MONITOR: Lazy<LogMonitorController> = Lazy::new(LogMonitorController::default);

pub fn global() -> &'static LogMonitorController {
    &LOG_MONITOR
}
