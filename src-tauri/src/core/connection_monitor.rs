use crate::core::{
    connection_metrics::{self, ConnectionMetricsEventPayload},
    handle,
};
use crate::process::AsyncHandler;
use crate::utils::connections_stream;
use crate::{Type, logging};
use clash_dtos::ConnectionId;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime::JoinHandle;

const MONITOR_RETRY_DELAY: Duration = Duration::from_secs(2);
const MONITOR_IDLE_POLL_INTERVAL: Duration = Duration::from_millis(500);
const MONITOR_STALE_TIMEOUT: Duration = Duration::from_secs(8);

pub struct ConnectionMonitorController {
    task: Arc<Mutex<Option<JoinHandle<()>>>>,
    connection_id: Arc<Mutex<Option<ConnectionId>>>,
    refs: Arc<Mutex<usize>>,
}

impl Default for ConnectionMonitorController {
    fn default() -> Self {
        Self {
            task: Arc::new(Mutex::new(None)),
            connection_id: Arc::new(Mutex::new(None)),
            refs: Arc::new(Mutex::new(0)),
        }
    }
}

impl ConnectionMonitorController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&self) {
        if handle::Handle::global().is_exiting() {
            return;
        }

        *self.refs.lock() += 1;

        let mut guard = self.task.lock();
        if guard.as_ref().is_some_and(|t| !t.inner().is_finished()) {
            return;
        }

        let conn_id = Arc::clone(&self.connection_id);
        let task = AsyncHandler::spawn(move || async move {
            loop {
                if handle::Handle::global().is_exiting() {
                    break;
                }

                let stream_result = connections_stream::connect_connections_stream().await;
                let mut stream = match stream_result {
                    Ok(s) => s,
                    Err(err) => {
                        logging!(
                            debug,
                            Type::Core,
                            "Connection monitor stream connect failed, retrying: {err}"
                        );
                        tokio::time::sleep(MONITOR_RETRY_DELAY).await;
                        continue;
                    }
                };

                *conn_id.lock() = Some(stream.connection_id);

                loop {
                    let state = stream
                        .next_event(MONITOR_IDLE_POLL_INTERVAL, MONITOR_STALE_TIMEOUT, || {
                            handle::Handle::global().is_exiting()
                        })
                        .await;

                    match state {
                        connections_stream::StreamConsumeState::Event(event) => {
                            let raw = serde_json::to_value(&event.snapshot).unwrap_or_default();
                            let metrics = connection_metrics::ingest_connection_metrics_snapshot(&event.snapshot).await;
                            handle::Handle::send_connection_metrics(ConnectionMetricsEventPayload { metrics, raw });
                        }
                        connections_stream::StreamConsumeState::Stale => {
                            logging!(debug, Type::Core, "Connection monitor stream stale, reconnecting");
                            break;
                        }
                        connections_stream::StreamConsumeState::Closed
                        | connections_stream::StreamConsumeState::ExitRequested => {
                            break;
                        }
                    }
                }

                disconnect(&conn_id).await;

                if handle::Handle::global().is_exiting() {
                    break;
                }

                tokio::time::sleep(MONITOR_RETRY_DELAY).await;
            }

            *conn_id.lock() = None;
        });

        *guard = Some(task);
    }

    pub fn stop(&self) {
        let should_stop = {
            let mut refs = self.refs.lock();
            if *refs == 0 {
                return;
            }
            *refs -= 1;
            *refs == 0
        };

        if !should_stop {
            return;
        }

        let task = self.task.lock().take();
        let conn_id = Arc::clone(&self.connection_id);

        AsyncHandler::spawn(move || async move {
            if let Some(task) = task {
                task.abort();
                let _ = task.await;
            }
            disconnect(&conn_id).await;
            connection_metrics::reset_connection_metrics().await;
        });
    }

    pub fn is_running(&self) -> bool {
        self.task.lock().as_ref().is_some_and(|t| !t.inner().is_finished())
    }
}

async fn disconnect(conn_id: &Arc<Mutex<Option<ConnectionId>>>) {
    let id = conn_id.lock().take();
    if let Some(id) = id {
        connections_stream::disconnect_connection(id).await;
    }
}

static CONNECTION_MONITOR: Lazy<ConnectionMonitorController> = Lazy::new(ConnectionMonitorController::default);

pub fn global() -> &'static ConnectionMonitorController {
    &CONNECTION_MONITOR
}
