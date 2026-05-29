/**
 * 流量填充模块
 * 
 * 功能：
 * 1. 填充数据生成 - 生成随机加密填充数据
 * 2. 智能填充算法 - 根据流量动态调整填充
 * 3. 填充调度器 - 定时发送填充数据
 * 4. 性能控制 - 限制资源占用和自动降级
 */

use anyhow::Result;
use crate::config::IClashTemp;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, Notify, RwLock};
use tokio::task::JoinHandle;
use tokio::time;

/// 流量填充配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficPaddingConfig {
    /// 启用填充
    pub enabled: bool,
    /// 最小填充大小（字节）
    pub min_size: usize,
    /// 最大填充大小（字节）
    pub max_size: usize,
    /// 加密填充数据
    pub encrypt: bool,
    /// 填充强度
    pub intensity: PaddingIntensity,
    /// 填充频率
    pub frequency: PaddingFrequency,
    /// 填充时机
    pub timing: PaddingTiming,
    /// 智能填充
    pub smart_padding: bool,
    /// 性能控制
    pub performance_control: PerformanceControl,
}

impl Default for TrafficPaddingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_size: 512,
            max_size: 4096,
            encrypt: true,
            intensity: PaddingIntensity::Medium,
            frequency: PaddingFrequency {
                freq_type: FrequencyType::Time,
                interval: 10,
            },
            timing: PaddingTiming::Random,
            smart_padding: true,
            performance_control: PerformanceControl::default(),
        }
    }
}

/// 填充强度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaddingIntensity {
    Low,
    Medium,
    High,
    Custom(f32),
}

impl PaddingIntensity {
    pub fn as_multiplier(&self) -> f32 {
        match self {
            PaddingIntensity::Low => 0.5,
            PaddingIntensity::Medium => 1.0,
            PaddingIntensity::High => 2.0,
            PaddingIntensity::Custom(m) => *m,
        }
    }
}

/// 填充频率
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaddingFrequency {
    pub freq_type: FrequencyType,
    pub interval: u64,
}

/// 频率类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrequencyType {
    Time,      // 每 N 秒
    Request,   // 每 N 请求
    Random,    // 随机
}

/// 填充时机
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaddingTiming {
    Before,    // 请求前
    After,     // 请求后
    Random,    // 随机
}

/// 性能控制
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceControl {
    /// 最大带宽（字节/秒）
    pub max_bandwidth: usize,
    /// 最大 CPU 使用率（%）
    pub max_cpu_usage: f32,
    /// 最大内存（字节）
    pub max_memory: usize,
    /// 自动降级
    pub auto_downgrade: bool,
}

impl Default for PerformanceControl {
    fn default() -> Self {
        Self {
            max_bandwidth: 1024 * 1024, // 1 MB/s
            max_cpu_usage: 5.0,          // 5%
            max_memory: 10 * 1024 * 1024, // 10 MB
            auto_downgrade: true,
        }
    }
}

/// 填充统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaddingStats {
    /// 填充次数
    pub padding_count: u64,
    /// 填充总大小（字节）
    pub total_padding_size: u64,
    /// 带宽占用（字节/秒）
    pub bandwidth_usage: f32,
    /// CPU 占用（%）
    pub cpu_usage: f32,
    /// 内存占用（字节）
    pub memory_usage: usize,
    /// 最后填充时间
    pub last_padding_time: i64,
}

impl Default for PaddingStats {
    fn default() -> Self {
        Self {
            padding_count: 0,
            total_padding_size: 0,
            bandwidth_usage: 0.0,
            cpu_usage: 0.0,
            memory_usage: 0,
            last_padding_time: 0,
        }
    }
}

/// 填充调度器
pub struct PaddingScheduler {
    config: Arc<RwLock<TrafficPaddingConfig>>,
    stats: Arc<RwLock<PaddingStats>>,
    running: Arc<AtomicBool>,
    padding_count: Arc<AtomicU64>,
    total_size: Arc<AtomicU64>,
    shutdown: Arc<Mutex<Option<Arc<Notify>>>>,
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl PaddingScheduler {
    /// 创建新的填充调度器
    pub fn new(config: TrafficPaddingConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            stats: Arc::new(RwLock::new(PaddingStats::default())),
            running: Arc::new(AtomicBool::new(false)),
            padding_count: Arc::new(AtomicU64::new(0)),
            total_size: Arc::new(AtomicU64::new(0)),
            shutdown: Arc::new(Mutex::new(None)),
            handle: Arc::new(Mutex::new(None)),
        }
    }

    /// 启动填充调度
    pub async fn start(&self) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            log::warn!("Padding scheduler is already running");
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);
        log::info!("🎯 Starting traffic padding scheduler");

        let config = self.config.clone();
        let stats = self.stats.clone();
        let running = self.running.clone();
        let padding_count = self.padding_count.clone();
        let total_size = self.total_size.clone();

        let shutdown_token = Arc::new(Notify::new());
        {
            let mut guard = self.shutdown.lock().await;
            *guard = Some(shutdown_token.clone());
        }

        let handle_slot = self.handle.clone();

        let join = tokio::spawn(async move {
            Self::schedule_loop(
                config,
                stats,
                running,
                padding_count,
                total_size,
                shutdown_token,
            )
            .await;
        });

        let mut handle_guard = handle_slot.lock().await;
        *handle_guard = Some(join);

        Ok(())
    }

    /// 停止填充调度
    pub async fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(token) = self.shutdown.lock().await.take() {
            token.notify_waiters();
        }

        if let Some(handle) = self.handle.lock().await.take() {
            if let Err(e) = handle.await {
                log::warn!("Padding scheduler task join failed: {}", e);
            }
        }

        // 重置计数与统计，避免下次启动继承旧值
        self.padding_count.store(0, Ordering::SeqCst);
        self.total_size.store(0, Ordering::SeqCst);
        let mut stats_guard = self.stats.write().await;
        *stats_guard = PaddingStats::default();

        log::info!("🛑 Stopping traffic padding scheduler");
    }

    /// 调度循环
    async fn schedule_loop(
        config: Arc<RwLock<TrafficPaddingConfig>>,
        stats: Arc<RwLock<PaddingStats>>,
        running: Arc<AtomicBool>,
        padding_count: Arc<AtomicU64>,
        total_size: Arc<AtomicU64>,
        shutdown: Arc<Notify>,
    ) {
        while running.load(Ordering::SeqCst) {
            let cfg = config.read().await.clone();

            if !cfg.enabled {
                tokio::select! {
                    _ = shutdown.notified() => break,
                    _ = time::sleep(Duration::from_secs(1)) => {},
                }
                continue;
            }

            // 检查性能限制
            if !Self::check_performance_limits(&cfg).await {
                if cfg.performance_control.auto_downgrade {
                    log::warn!("⚠️ Performance limit reached, auto-downgrading");
                    tokio::select! {
                        _ = shutdown.notified() => break,
                        _ = time::sleep(Duration::from_secs(cfg.frequency.interval * 2)) => {},
                    }
                    continue;
                }
            }

            // 计算填充大小
            let size = if cfg.smart_padding {
                Self::calculate_smart_padding_size(&cfg).await
            } else {
                Self::calculate_random_padding_size(&cfg)
            };

            // 生成并发送填充数据
            match Self::generate_and_send_padding(size, cfg.encrypt).await {
                Ok(actual_size) => {
                    // 更新统计
                    padding_count.fetch_add(1, Ordering::SeqCst);
                    total_size.fetch_add(actual_size as u64, Ordering::SeqCst);

                    let mut stats_guard = stats.write().await;
                    stats_guard.padding_count = padding_count.load(Ordering::SeqCst);
                    stats_guard.total_padding_size = total_size.load(Ordering::SeqCst);
                    stats_guard.last_padding_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64;

                    log::trace!("✅ Padding sent: {} bytes", actual_size);
                }
                Err(e) => {
                    log::error!("Failed to send padding: {}", e);
                }
            }

            // 等待下一次填充
            let interval = Self::calculate_interval(&cfg);
            tokio::select! {
                _ = shutdown.notified() => break,
                _ = time::sleep(interval) => {},
            }
        }

        log::info!("Padding scheduler loop stopped");
    }

    /// 生成随机填充数据
    fn generate_padding_data(size: usize, encrypt: bool) -> Result<Vec<u8>> {
        let mut rng = rand::thread_rng();
        let mut data = vec![0u8; size];
        rng.fill(&mut data[..]);

        if encrypt {
            // 简单的 XOR 加密（实际应用中应使用 AES-256-GCM）
            let key: u8 = rng.r#gen();
            for byte in data.iter_mut() {
                *byte ^= key;
            }
        }

        Ok(data)
    }

    /// 计算随机填充大小
    fn calculate_random_padding_size(config: &TrafficPaddingConfig) -> usize {
        let mut rng = rand::thread_rng();
        let base_size = rng.gen_range(config.min_size..=config.max_size);
        let multiplier = config.intensity.as_multiplier();
        (base_size as f32 * multiplier) as usize
    }

    /// 计算智能填充大小
    async fn calculate_smart_padding_size(config: &TrafficPaddingConfig) -> usize {
        // 模拟流量监控数据
        let current_traffic = 100_000.0_f32; // 100 KB/s
        let network_latency = 50.0_f32;      // 50 ms
        let bandwidth_usage = 0.3_f32;       // 30%

        let base_size = (config.min_size + config.max_size) / 2;

        // 流量越小，填充越多
        let traffic_factor = 1.0_f32 - (current_traffic / 1_000_000.0_f32).min(1.0_f32);

        // 延迟越高，填充越少
        let latency_factor = 1.0_f32 - (network_latency / 1000.0_f32).min(1.0_f32);

        // 带宽使用率越高，填充越少
        let bandwidth_factor = 1.0_f32 - bandwidth_usage.min(1.0_f32);

        // 应用强度
        let multiplier = config.intensity.as_multiplier();

        let size = (base_size as f32)
            * traffic_factor
            * latency_factor
            * bandwidth_factor
            * multiplier;

        size.max(config.min_size as f32) as usize
    }

    /// 检查性能限制
    async fn check_performance_limits(_config: &TrafficPaddingConfig) -> bool {
        // 简化实现：总是返回 true
        // 实际应用中应检查实际的 CPU、内存和带宽使用情况
        true
    }

    /// 计算等待间隔
    fn calculate_interval(config: &TrafficPaddingConfig) -> Duration {
        match config.frequency.freq_type {
            FrequencyType::Time => Duration::from_secs(config.frequency.interval),
            FrequencyType::Random => {
                let mut rng = rand::thread_rng();
                let random_interval = rng.gen_range(
                    config.frequency.interval / 2..=config.frequency.interval * 2,
                );
                Duration::from_secs(random_interval)
            }
            FrequencyType::Request => {
                // 简化实现：使用固定间隔
                Duration::from_secs(config.frequency.interval)
            }
        }
    }

    /// 生成并发送填充数据
    async fn generate_and_send_padding(size: usize, encrypt: bool) -> Result<usize> {
        let data = Self::generate_padding_data(size, encrypt)?;
        Self::send_padding_via_mixed_proxy(&data).await
    }

    async fn send_padding_via_mixed_proxy(data: &[u8]) -> Result<usize> {
        let clash_cfg = IClashTemp::new().await;
        let mixed_port = clash_cfg.get_mixed_port();
        let addr = format!("127.0.0.1:{mixed_port}");

        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| anyhow::anyhow!("connect {} failed: {}", addr, e))?;

        stream.write_all(data).await?;
        stream.flush().await?;

        Ok(data.len())
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> PaddingStats {
        self.stats.read().await.clone()
    }

    /// 重置统计信息
    pub async fn reset_stats(&self) {
        self.padding_count.store(0, Ordering::SeqCst);
        self.total_size.store(0, Ordering::SeqCst);

        let mut stats = self.stats.write().await;
        *stats = PaddingStats::default();

        log::info!("📊 Padding stats reset");
    }

    /// 更新配置
    pub async fn update_config(&self, config: TrafficPaddingConfig) {
        let mut cfg = self.config.write().await;
        *cfg = config;
        log::info!("📝 Padding config updated");
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 获取当前配置
    pub async fn get_config(&self) -> TrafficPaddingConfig {
        self.config.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_padding_intensity_multiplier() {
        assert_eq!(PaddingIntensity::Low.as_multiplier(), 0.5);
        assert_eq!(PaddingIntensity::Medium.as_multiplier(), 1.0);
        assert_eq!(PaddingIntensity::High.as_multiplier(), 2.0);
        assert_eq!(PaddingIntensity::Custom(1.5).as_multiplier(), 1.5);
    }

    #[test]
    fn test_generate_padding_data() {
        let data = PaddingScheduler::generate_padding_data(1024, false).unwrap();
        assert_eq!(data.len(), 1024);
    }

    #[test]
    fn test_generate_encrypted_padding_data() {
        let data = PaddingScheduler::generate_padding_data(1024, true).unwrap();
        assert_eq!(data.len(), 1024);
    }

    #[test]
    fn test_calculate_random_padding_size() {
        let config = TrafficPaddingConfig {
            min_size: 512,
            max_size: 4096,
            intensity: PaddingIntensity::Medium,
            ..Default::default()
        };

        let size = PaddingScheduler::calculate_random_padding_size(&config);
        assert!(size >= 512 && size <= 4096);
    }

    #[tokio::test]
    async fn test_calculate_smart_padding_size() {
        let config = TrafficPaddingConfig {
            min_size: 512,
            max_size: 4096,
            intensity: PaddingIntensity::Medium,
            smart_padding: true,
            ..Default::default()
        };

        let size = PaddingScheduler::calculate_smart_padding_size(&config).await;
        assert!(size >= 512);
    }

    #[tokio::test]
    async fn test_padding_scheduler_creation() {
        let config = TrafficPaddingConfig::default();
        let scheduler = PaddingScheduler::new(config);

        assert!(!scheduler.is_running());
    }

    #[tokio::test]
    async fn test_padding_scheduler_start_stop() {
        let config = TrafficPaddingConfig::default();
        let scheduler = PaddingScheduler::new(config);

        scheduler.start().await.unwrap();
        assert!(scheduler.is_running());

        scheduler.stop().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!scheduler.is_running());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let config = TrafficPaddingConfig::default();
        let scheduler = PaddingScheduler::new(config);

        let stats = scheduler.get_stats().await;
        assert_eq!(stats.padding_count, 0);
        assert_eq!(stats.total_padding_size, 0);
    }

    #[tokio::test]
    async fn test_reset_stats() {
        let config = TrafficPaddingConfig::default();
        let scheduler = PaddingScheduler::new(config);

        scheduler.reset_stats().await;

        let stats = scheduler.get_stats().await;
        assert_eq!(stats.padding_count, 0);
        assert_eq!(stats.total_padding_size, 0);
    }

    #[tokio::test]
    async fn test_update_config() {
        let config = TrafficPaddingConfig::default();
        let scheduler = PaddingScheduler::new(config);

        let new_config = TrafficPaddingConfig {
            enabled: true,
            min_size: 1024,
            ..Default::default()
        };

        scheduler.update_config(new_config).await;
    }

    #[test]
    fn test_calculate_interval_time() {
        let config = TrafficPaddingConfig {
            frequency: PaddingFrequency {
                freq_type: FrequencyType::Time,
                interval: 10,
            },
            ..Default::default()
        };

        let interval = PaddingScheduler::calculate_interval(&config);
        assert_eq!(interval, Duration::from_secs(10));
    }

    #[test]
    fn test_calculate_interval_random() {
        let config = TrafficPaddingConfig {
            frequency: PaddingFrequency {
                freq_type: FrequencyType::Random,
                interval: 10,
            },
            ..Default::default()
        };

        let interval = PaddingScheduler::calculate_interval(&config);
        assert!(interval >= Duration::from_secs(5) && interval <= Duration::from_secs(20));
    }
}
