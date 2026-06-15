use crate::core::{connection_metrics, connection_monitor, handle};
use crate::process::AsyncHandler;
use crate::utils::tray_speed;
use crate::{Type, logging};
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::async_runtime::JoinHandle;

/// 托盘速率流在此时间内收不到统一指标更新时，降级到 0/0。
const TRAY_SPEED_STALE_TIMEOUT: Duration = Duration::from_secs(5);

/// macOS 托盘速率任务控制器。
#[derive(Clone)]
pub struct TraySpeedController {
    speed_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    monitor_acquired: Arc<AtomicBool>,
}

impl Default for TraySpeedController {
    fn default() -> Self {
        Self {
            speed_task: Arc::new(Mutex::new(None)),
            monitor_acquired: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl TraySpeedController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_task(&self, enabled: bool) {
        if enabled {
            self.start_task();
        } else {
            self.stop_task();
        }
    }

    /// 启动托盘速率采集后台任务（基于 Rust 统一连接指标流）。
    fn start_task(&self) {
        if handle::Handle::global().is_exiting() {
            return;
        }

        // 关键步骤：托盘不可用时不启动速率任务，避免无效连接重试。
        if !Self::has_main_tray() {
            logging!(warn, Type::Tray, "托盘不可用，跳过启动托盘速率任务");
            return;
        }

        let mut guard = self.speed_task.lock();
        if guard.as_ref().is_some_and(|task| !task.inner().is_finished()) {
            return;
        }

        connection_monitor::global().start();
        self.monitor_acquired.store(true, Ordering::SeqCst);

        let monitor_acquired = Arc::clone(&self.monitor_acquired);
        let task = AsyncHandler::spawn(move || async move {
            let mut metrics_rx = connection_metrics::subscribe_connection_metrics();

            loop {
                if handle::Handle::global().is_exiting() {
                    break;
                }

                if !Self::has_main_tray() {
                    logging!(warn, Type::Tray, "托盘已不可用，停止托盘速率任务");
                    break;
                }

                match tokio::time::timeout(TRAY_SPEED_STALE_TIMEOUT, metrics_rx.changed()).await {
                    Ok(Ok(())) => {
                        let snapshot = metrics_rx.borrow().clone();
                        if snapshot.stale {
                            Self::apply_tray_speed(0, 0);
                        } else {
                            Self::apply_tray_speed(snapshot.traffic.upload_speed, snapshot.traffic.download_speed);
                        }
                    }
                    Ok(Err(_)) => {
                        Self::apply_tray_speed(0, 0);
                        break;
                    }
                    Err(_) => {
                        Self::apply_tray_speed(0, 0);
                    }
                }
            }

            if monitor_acquired.swap(false, Ordering::SeqCst) {
                connection_monitor::global().stop();
            }
        });

        *guard = Some(task);
    }

    /// 停止托盘速率采集后台任务并清除速率显示。
    fn stop_task(&self) {
        let task = self.speed_task.lock().take();
        let monitor_acquired = Arc::clone(&self.monitor_acquired);

        AsyncHandler::spawn(move || async move {
            if let Some(task) = task {
                task.abort();
                let _ = task.await;
            }
            if monitor_acquired.swap(false, Ordering::SeqCst) {
                connection_monitor::global().stop();
            }
        });

        let app_handle = handle::Handle::app_handle();
        if let Some(tray) = app_handle.tray_by_id("main") {
            let result = tray.with_inner_tray_icon(|inner| {
                if let Some(status_item) = inner.ns_status_item() {
                    tray_speed::clear_speed_attributed_title(&status_item);
                }
            });
            if let Err(err) = result {
                logging!(warn, Type::Tray, "清除富文本速率失败: {err}");
            }
        }
    }

    fn has_main_tray() -> bool {
        handle::Handle::app_handle().tray_by_id("main").is_some()
    }

    fn apply_tray_speed(up: u64, down: u64) {
        let app_handle = handle::Handle::app_handle();
        if let Some(tray) = app_handle.tray_by_id("main") {
            let result = tray.with_inner_tray_icon(move |inner| {
                if let Some(status_item) = inner.ns_status_item() {
                    tray_speed::set_speed_attributed_title(&status_item, up, down);
                }
            });
            if let Err(err) = result {
                logging!(warn, Type::Tray, "设置富文本速率失败: {err}");
            }
        }
    }
}
