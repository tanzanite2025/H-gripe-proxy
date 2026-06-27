use crate::{config::Config, core::tray::Tray, utils::dirs};
use anyhow::{Result, anyhow, bail};
use backon::{ConstantBuilder, Retryable as _};
use clash_verge_logging::{Type, logging, logging_error};
use once_cell::sync::Lazy;
use std::{borrow::Cow, path::Path, process::Command as StdCommand, time::Duration};
use tokio::sync::Mutex;
use windows::Win32::Foundation::ERROR_PIPE_BUSY;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStatus {
    Ready,
    NeedsReinstall,
    InstallRequired,
    UninstallRequired,
    ReinstallRequired,
    ForceReinstallRequired,
    Unavailable(String),
}

#[derive(Clone)]
pub struct ServiceManager(ServiceStatus);

fn uninstall_service() -> Result<()> {
    logging!(info, Type::Service, "uninstall service");

    use deelevate::{PrivilegeLevel, Token};
    use runas::Command as RunasCommand;
    use std::os::windows::process::CommandExt as _;

    let binary_path = dirs::service_path()?;
    let uninstall_path = binary_path.with_file_name("clash-verge-service-uninstall.exe");

    if !uninstall_path.exists() {
        bail!(format!("uninstaller not found: {uninstall_path:?}"));
    }

    let token = Token::with_current_process()?;
    let level = token.privilege_level()?;
    let status = match level {
        PrivilegeLevel::NotPrivileged => RunasCommand::new(uninstall_path).show(false).status()?,
        _ => StdCommand::new(uninstall_path).creation_flags(0x08000000).status()?,
    };

    if !status.success() {
        bail!(
            "failed to uninstall service with status {}",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

fn install_service() -> Result<()> {
    use std::process::Output;
    logging!(info, Type::Service, "install service");

    use deelevate::{PrivilegeLevel, Token};
    use runas::Command as RunasCommand;
    use std::os::windows::process::CommandExt as _;

    let binary_path = dirs::service_path()?;
    let install_path = binary_path.with_file_name("clash-verge-service-install.exe");

    if !install_path.exists() {
        bail!(format!("installer not found: {install_path:?}"));
    }

    let token = Token::with_current_process()?;
    let level = token.privilege_level()?;
    let output = match level {
        PrivilegeLevel::NotPrivileged => {
            let status = RunasCommand::new(&install_path).show(false).status()?;
            Output {
                status,
                stdout: Vec::new(),
                stderr: Vec::new(),
            }
        }
        _ => {
            // StdCommand returns Output directly
            StdCommand::new(&install_path).creation_flags(0x08000000).output()?
        }
    };

    if let Some((code, err)) = check_output_error(&output) {
        logging!(
            error,
            Type::Service,
            "failed to install service code: {}, details: {}",
            code,
            err
        );
        bail!("failed to install service code: {}, details: {}", code, err);
    }

    Ok(())
}

fn check_output_error(output: &std::process::Output) -> Option<(i32, Cow<'_, str>)> {
    if output.status.success() {
        return None;
    }
    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        return Some((code, stderr));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        return Some((code, stdout));
    }
    Some((code, Cow::Borrowed("Unknown error")))
}

fn reinstall_service() -> Result<()> {
    logging!(info, Type::Service, "reinstall service");

    // 先卸载服务
    if let Err(err) = uninstall_service() {
        logging!(warn, Type::Service, "failed to uninstall service: {}", err);
    }

    // 再安装服务
    match install_service() {
        Ok(_) => Ok(()),
        Err(err) => {
            bail!(format!("failed to install service: {err}"))
        }
    }
}

/// 强制重装服务（UI修复按钮）
fn force_reinstall_service() -> Result<()> {
    logging!(info, Type::Service, "用户请求强制重装服务");
    reinstall_service().map_err(|err| {
        logging!(error, Type::Service, "强制重装服务失败: {}", err);
        err
    })
}

/// 检查服务是否正在运行
pub async fn is_service_available() -> Result<()> {
    if let Err(e) = Path::metadata(clash_verge_service_ipc::IPC_PATH.as_ref()) {
        if e.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32) {
            logging!(
                debug,
                Type::Service,
                "Service IPC path is busy but available, continuing to connect"
            );
        } else {
            let verge = Config::verge().await;
            let verge_last = verge.latest_arc();
            let is_enable = verge_last.enable_tun_mode.unwrap_or(false);
            if is_enable {
                logging!(warn, Type::Service, "Some issue with service IPC Path: {}", e);
            }
            return Err(e.into());
        }
    }
    clash_verge_service_ipc::connect().await?;
    Ok(())
}

pub async fn wait_and_check_service_available(status: &mut ServiceManager) -> Result<()> {
    wait_for_service_ipc(status, "Waiting for service to be available").await
}

async fn wait_and_check_service_version(status: &mut ServiceManager) -> Result<()> {
    wait_and_check_service_available(status).await?;

    if clash_verge_service_ipc::is_reinstall_service_needed().await {
        logging!(info, Type::Service, "服务版本不匹配，执行重装流程");
        reinstall_service()?;
        wait_and_check_service_available(status).await?;
    }

    Ok(())
}

async fn wait_for_service_ipc(status: &mut ServiceManager, reason: &str) -> Result<()> {
    status.0 = ServiceStatus::Unavailable(reason.into());
    let config = ServiceManager::config();

    let backoff = ConstantBuilder::default()
        .with_delay(config.retry_delay)
        .with_max_times(config.max_retries);

    let result = (|| async {
        if is_service_ipc_path_exists() {
            clash_verge_service_ipc::connect().await?;
            Ok(())
        } else {
            Err(anyhow!("IPC path not ready"))
        }
    })
    .retry(backoff)
    .await;

    if result.is_ok() {
        status.0 = ServiceStatus::Ready;
    }

    result
}

pub fn is_service_ipc_path_exists() -> bool {
    match Path::metadata(clash_verge_service_ipc::IPC_PATH.as_ref()) {
        Ok(_) => true,
        Err(err) if err.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32) => true,
        Err(_) => false,
    }
}

impl ServiceManager {
    pub fn default() -> Self {
        Self(ServiceStatus::Unavailable("Need Checks".into()))
    }

    pub const fn config() -> clash_verge_service_ipc::IpcConfig {
        clash_verge_service_ipc::IpcConfig {
            default_timeout: Duration::from_millis(150),
            retry_delay: Duration::from_millis(250),
            max_retries: 20,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        if let Err(e) = clash_verge_service_ipc::connect().await {
            self.0 = ServiceStatus::Unavailable("服务连接失败: {e}".to_string());
            return Err(e);
        }
        Ok(())
    }

    pub fn current(&self) -> ServiceStatus {
        self.0.clone()
    }

    pub async fn refresh(&mut self) -> Result<()> {
        let status = self.check_service_comprehensive().await;
        self.0 = status.clone();
        logging_error!(Type::Service, self.handle_service_status(&status).await);
        Ok(())
    }

    /// 综合服务状态检查（一次性完成所有检查）
    pub async fn check_service_comprehensive(&self) -> ServiceStatus {
        if clash_verge_service_ipc::is_reinstall_service_needed().await {
            ServiceStatus::NeedsReinstall
        } else {
            ServiceStatus::Ready
        }
    }

    /// 根据服务状态执行相应操作
    pub async fn handle_service_status(&mut self, status: &ServiceStatus) -> Result<()> {
        match status {
            ServiceStatus::Ready => {
                logging!(info, Type::Service, "服务就绪，直接启动");
                self.0 = ServiceStatus::Ready;
            }
            ServiceStatus::NeedsReinstall | ServiceStatus::ReinstallRequired => {
                logging!(info, Type::Service, "服务需要重装，执行重装流程");
                reinstall_service()?;
                wait_and_check_service_available(self).await?;
            }
            ServiceStatus::ForceReinstallRequired => {
                logging!(info, Type::Service, "服务需要强制重装，执行强制重装流程");
                force_reinstall_service()?;
                wait_and_check_service_available(self).await?;
            }
            ServiceStatus::InstallRequired => {
                logging!(info, Type::Service, "需要安装服务，执行安装流程");
                install_service()?;
                wait_and_check_service_version(self).await?;
            }
            ServiceStatus::UninstallRequired => {
                logging!(info, Type::Service, "服务需要卸载，执行卸载流程");
                uninstall_service()?;
                self.0 = ServiceStatus::Unavailable("Service Uninstalled".into());
            }
            ServiceStatus::Unavailable(reason) => {
                logging!(info, Type::Service, "服务不可用: {}，将使用Sidecar模式", reason);
                self.0 = ServiceStatus::Unavailable(reason.clone());
                return Err(anyhow::anyhow!("服务不可用: {}", reason));
            }
        }

        // 防止服务安装成功后，内核未完全启动导致系统托盘无法获取代理节点信息
        Tray::global().update_menu().await?;
        Ok(())
    }
}

pub static SERVICE_MANAGER: Lazy<Mutex<ServiceManager>> = Lazy::new(|| Mutex::new(ServiceManager::default()));
