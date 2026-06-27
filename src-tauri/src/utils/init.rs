// #[cfg(not(feature = "tracing"))]
use crate::{
    config::{
        Config, DOMESTIC_DOH_NAMESERVERS, ENCRYPTED_BOOTSTRAP_NAMESERVERS, FOREIGN_DOH_NAMESERVERS, IClashTemp,
        IProfiles, IVerge, value_sequence,
    },
    constants, logging,
    process::AsyncHandler,
    utils::{
        dirs::{self, PathBufExec as _},
        help, tmpl,
    },
};
use anyhow::Result;
use chrono::{Local, TimeZone as _};
use clash_verge_logging::Type;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    str::FromStr as _,
};
use tokio::fs;
use tokio::fs::DirEntry;
use tokio::process::Command;

const STARTUP_SCRIPT_AUTH_FILE: &str = ".startup_script_authorization.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
struct StartupScriptAuthorization {
    canonical_path: String,
    sha256: String,
}

fn startup_script_authorization_path() -> Result<PathBuf> {
    Ok(dirs::app_home_dir()?.join(STARTUP_SCRIPT_AUTH_FILE))
}

async fn build_startup_script_authorization(script_path: &Path) -> Result<StartupScriptAuthorization> {
    let canonical_path = dunce::canonicalize(script_path)?;
    let content = fs::read(&canonical_path).await?;
    let sha256 = hex::encode(Sha256::digest(&content));
    Ok(StartupScriptAuthorization {
        canonical_path: canonical_path.to_string_lossy().into_owned().into(),
        sha256: sha256.into(),
    })
}

async fn startup_script_authorization_matches(
    script_path: &Path,
    authorization: &StartupScriptAuthorization,
) -> Result<bool> {
    let current = build_startup_script_authorization(script_path).await?;
    Ok(current.canonical_path == authorization.canonical_path && current.sha256 == authorization.sha256)
}

async fn load_startup_script_authorization() -> Result<Option<StartupScriptAuthorization>> {
    let auth_path = startup_script_authorization_path()?;
    if !auth_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(auth_path).await?;
    Ok(Some(serde_json::from_str(&content)?))
}

pub async fn authorize_startup_script(script_path: String) -> Result<()> {
    let script_path = PathBuf::from(script_path);
    validate_startup_script_path(&script_path)?;

    let authorization = build_startup_script_authorization(&script_path).await?;
    let auth_path = startup_script_authorization_path()?;
    if let Some(parent) = auth_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(auth_path, serde_json::to_string_pretty(&authorization)?).await?;
    Ok(())
}

pub async fn clear_startup_script_authorization() -> Result<()> {
    let auth_path = startup_script_authorization_path()?;
    if auth_path.exists() {
        auth_path.remove_if_exists().await?;
    }
    Ok(())
}

fn validate_startup_script_path(script_path: &Path) -> Result<&'static str> {
    let extension = script_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let shell_type = match extension.as_str() {
        "sh" => "bash",
        "ps1" | "bat" => "powershell",
        _ => {
            return Err(anyhow::anyhow!(
                "unsupported script extension: {}",
                script_path.display()
            ));
        }
    };

    if !script_path.exists() {
        return Err(anyhow::anyhow!("script not found: {}", script_path.display()));
    }
    if !script_path.is_file() {
        return Err(anyhow::anyhow!(
            "startup script path is not a file: {}",
            script_path.display()
        ));
    }
    Ok(shell_type)
}

async fn delete_snapshot_logs(log_dir: &Path) -> Result<()> {
    let temp_dirs = [
        log_dir.join("temp"),
        log_dir.join("service").join("temp"),
        log_dir.join("sidecar").join("temp"),
    ];

    for temp_dir in temp_dirs.iter().filter(|d| d.exists()) {
        let mut entries = fs::read_dir(temp_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("log") {
                let _ = path.remove_if_exists().await;
                logging!(info, Type::Setup, "delete snapshot log file: {}", path.display());
            }
        }
    }

    Ok(())
}

// TODO flexi_logger 提供了最大保留天数，或许我们应该用内置删除log文件
/// 删除log文件
pub async fn delete_log() -> Result<()> {
    let log_dir = dirs::app_logs_dir()?;
    if !log_dir.exists() {
        return Ok(());
    }

    delete_snapshot_logs(&log_dir).await?;

    let auto_log_clean = {
        let verge = Config::verge().await;
        let verge = verge.data_arc();
        verge.auto_log_clean.unwrap_or(0)
    };

    // 1: 1天, 2: 7天, 3: 30天, 4: 90天
    let day = match auto_log_clean {
        1 => 1,
        2 => 7,
        3 => 30,
        4 => 90,
        _ => return Ok(()),
    };

    logging!(info, Type::Setup, "try to delete log files, day: {}", day);

    // %Y-%m-%d to NaiveDateTime
    let parse_time_str = |s: &str| {
        let sa: Vec<&str> = s.split('-').collect();
        if sa.len() != 4 {
            return Err(anyhow::anyhow!("invalid time str"));
        }

        let year = i32::from_str(sa[0])?;
        let month = u32::from_str(sa[1])?;
        let day = u32::from_str(sa[2])?;
        let time = chrono::NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| anyhow::anyhow!("invalid time str"))?
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("invalid time str"))?;
        Ok(time)
    };

    let process_file = async move |file: DirEntry| -> Result<()> {
        let file_name = file.file_name();
        let file_name = file_name.to_str().unwrap_or_default();

        if file_name.ends_with(".log") {
            let now = Local::now();
            let created_time = parse_time_str(&file_name[0..file_name.len() - 4])?;
            let file_time = Local
                .from_local_datetime(&created_time)
                .single()
                .ok_or_else(|| anyhow::anyhow!("invalid local datetime"))?;

            let duration = now.signed_duration_since(file_time);
            if duration.num_days() > day {
                let _ = file.path().remove_if_exists().await;
                logging!(info, Type::Setup, "delete log file: {}", file_name);
            }
        }
        Ok(())
    };

    let mut log_read_dir = fs::read_dir(&log_dir).await?;
    while let Some(entry) = log_read_dir.next_entry().await? {
        std::mem::drop(process_file(entry).await);
    }

    let service_log_dir = log_dir.join("service");
    let mut service_log_read_dir = fs::read_dir(service_log_dir).await?;
    while let Some(entry) = service_log_read_dir.next_entry().await? {
        std::mem::drop(process_file(entry).await);
    }

    Ok(())
}

/// 初始化DNS配置文件
async fn init_dns_config() -> Result<()> {
    use serde_yaml_ng::Value;

    // 创建DNS子配置
    let dns_config = serde_yaml_ng::Mapping::from_iter([
        ("enable".into(), Value::Bool(true)),
        ("listen".into(), Value::String(":53".into())),
        ("enhanced-mode".into(), Value::String("fake-ip".into())),
        ("fake-ip-range".into(), Value::String("198.18.0.1/16".into())),
        ("fake-ip-filter-mode".into(), Value::String("blacklist".into())),
        ("prefer-h3".into(), Value::Bool(false)),
        ("respect-rules".into(), Value::Bool(false)),
        ("use-hosts".into(), Value::Bool(false)),
        ("use-system-hosts".into(), Value::Bool(false)),
        (
            "fake-ip-filter".into(),
            Value::Sequence(vec![
                Value::String("*.lan".into()),
                Value::String("*.local".into()),
                Value::String("*.arpa".into()),
                Value::String("time.*.com".into()),
                Value::String("ntp.*.com".into()),
                Value::String("time.*.com".into()),
                Value::String("+.market.xiaomi.com".into()),
                Value::String("localhost.ptlogin2.qq.com".into()),
                Value::String("*.msftncsi.com".into()),
                Value::String("www.msftconnecttest.com".into()),
            ]),
        ),
        (
            "default-nameserver".into(),
            Value::Sequence(value_sequence(ENCRYPTED_BOOTSTRAP_NAMESERVERS)),
        ),
        (
            "nameserver".into(),
            Value::Sequence(value_sequence(DOMESTIC_DOH_NAMESERVERS)),
        ),
        (
            "fallback".into(),
            Value::Sequence(value_sequence(FOREIGN_DOH_NAMESERVERS)),
        ),
        (
            "nameserver-policy".into(),
            Value::Mapping(serde_yaml_ng::Mapping::from_iter([
                (
                    "geosite:cn".into(),
                    Value::Sequence(value_sequence(DOMESTIC_DOH_NAMESERVERS)),
                ),
                (
                    "geosite:geolocation-!cn".into(),
                    Value::Sequence(value_sequence(FOREIGN_DOH_NAMESERVERS)),
                ),
            ])),
        ),
        (
            "proxy-server-nameserver".into(),
            Value::Sequence(value_sequence(DOMESTIC_DOH_NAMESERVERS)),
        ),
        ("direct-nameserver".into(), Value::Sequence(vec![])),
        ("direct-nameserver-follow-policy".into(), Value::Bool(false)),
        (
            "fallback-filter".into(),
            Value::Mapping(serde_yaml_ng::Mapping::from_iter([
                ("geoip".into(), Value::Bool(true)),
                ("geoip-code".into(), Value::String("CN".into())),
                (
                    "ipcidr".into(),
                    Value::Sequence(vec![
                        Value::String("240.0.0.0/4".into()),
                        Value::String("0.0.0.0/32".into()),
                    ]),
                ),
                (
                    "domain".into(),
                    Value::Sequence(vec![
                        Value::String("+.google.com".into()),
                        Value::String("+.facebook.com".into()),
                        Value::String("+.youtube.com".into()),
                    ]),
                ),
            ])),
        ),
    ]);

    // 获取默认DNS和host配置
    let default_dns_config = serde_yaml_ng::Mapping::from_iter([
        ("dns".into(), Value::Mapping(dns_config)),
        ("hosts".into(), Value::Mapping(serde_yaml_ng::Mapping::new())),
    ]);

    // 检查DNS配置文件是否存在
    let app_dir = dirs::app_home_dir()?;
    let dns_path = app_dir.join(constants::files::DNS_CONFIG);

    if !dns_path.exists() {
        logging!(info, Type::Setup, "Creating default DNS config file");
        help::save_yaml(
            &dns_path,
            &default_dns_config,
            Some("# Clash Verge Optimized DNS Config"),
        )
        .await?;
    }

    Ok(())
}

/// 确保目录结构存在
async fn ensure_directories() -> Result<()> {
    let directories = [
        ("app_home", dirs::app_home_dir()?),
        ("app_profiles", dirs::app_profiles_dir()?),
        ("app_logs", dirs::app_logs_dir()?),
    ];

    for (name, dir) in directories {
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create {} directory {:?}: {}", name, dir, e))?;
            logging!(info, Type::Setup, "Created {} directory: {:?}", name, dir);
        }
    }

    Ok(())
}

/// 初始化配置文件
async fn initialize_config_files() -> Result<()> {
    if let Ok(path) = dirs::clash_path()
        && !path.exists()
    {
        let template = IClashTemp::template().0;
        help::save_yaml(&path, &template, Some("# Clash Verge Optimized"))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create clash config: {}", e))?;
        logging!(info, Type::Setup, "Created clash config at {:?}", path);
    }

    if let Ok(path) = dirs::verge_path()
        && !path.exists()
    {
        let template = IVerge::template();
        help::save_yaml(&path, &template, Some("# Clash Verge Optimized"))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create verge config: {}", e))?;
        logging!(info, Type::Setup, "Created verge config at {:?}", path);
    }

    if let Ok(path) = dirs::profiles_path()
        && !path.exists()
    {
        let template = IProfiles::default();
        help::save_yaml(&path, &template, Some("# Clash Verge Optimized"))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create profiles config: {}", e))?;
        logging!(info, Type::Setup, "Created profiles config at {:?}", path);
    }

    if let Ok(path) = dirs::china_rules_path()
        && !path.exists()
    {
        fs::write(&path, tmpl::CHINA_RULES_TEMPLATE)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create china rules config: {}", e))?;
        logging!(info, Type::Setup, "Created china rules config at {:?}", path);
    }

    Ok(())
}

/// Initialize all the config files
/// before tauri setup
pub async fn init_config() -> Result<()> {
    // We do not need init_portable_flag here anymore due to lib.rs will to the things
    // let _ = dirs::init_portable_flag();

    // We do not need init_log here anymore due to resolve will to the things

    ensure_directories().await?;

    initialize_config_files().await?;

    AsyncHandler::spawn(|| async {
        if let Err(e) = delete_log().await {
            logging!(warn, Type::Setup, "Failed to clean old logs: {}", e);
        }
        logging!(info, Type::Setup, "后台日志清理任务完成");
    });

    if let Err(e) = init_dns_config().await {
        logging!(warn, Type::Setup, "DNS config initialization failed: {}", e);
    }

    Ok(())
}

/// initialize app resources
/// after tauri setup
pub async fn init_resources() -> Result<()> {
    let app_dir = dirs::app_home_dir()?;
    let res_dir = dirs::app_resources_dir()?;

    if !app_dir.exists() {
        std::mem::drop(fs::create_dir_all(&app_dir).await);
    }
    if !res_dir.exists() {
        std::mem::drop(fs::create_dir_all(&res_dir).await);
    }

    let legacy_file_list = [
        "Country.mmdb",
        "country.mmdb",
        "GeoLite2-Country.mmdb",
        "ASN.mmdb",
        "City.mmdb",
    ];
    let file_list = ["GeoLite2-ASN.mmdb", "GeoLite2-City.mmdb", "geoip.dat", "geosite.dat"];

    for file in legacy_file_list {
        let legacy_path = app_dir.join(file);
        if legacy_path.exists() {
            if let Err(error) = fs::remove_file(&legacy_path).await {
                logging!(
                    warn,
                    Type::Setup,
                    "failed to remove legacy resource '{}': {}",
                    file,
                    error
                );
            } else {
                logging!(info, Type::Setup, "removed legacy resource '{}'", file);
            }
        }
    }

    // copy the resource file
    // if the source file is newer than the destination file, copy it over
    for file in file_list.iter() {
        let src_path = res_dir.join(file);
        let dest_path = app_dir.join(file);

        if src_path.exists() && !dest_path.exists() {
            handle_copy(&src_path, &dest_path, file).await;
            continue;
        }

        let src_modified = fs::metadata(&src_path).await.and_then(|m| m.modified());
        let dest_modified = fs::metadata(&dest_path).await.and_then(|m| m.modified());

        match (src_modified, dest_modified) {
            (Ok(src_modified), Ok(dest_modified)) => {
                if src_modified > dest_modified {
                    handle_copy(&src_path, &dest_path, file).await;
                }
            }
            _ => {
                logging!(debug, Type::Setup, "failed to get modified '{}'", file);
                handle_copy(&src_path, &dest_path, file).await;
            }
        };
    }

    Ok(())
}

/// initialize url scheme
pub fn init_scheme() -> Result<()> {
    use tauri::utils::platform::current_exe;
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    let app_exe = current_exe()?;
    let app_exe = dunce::canonicalize(app_exe)?;
    let app_exe = app_exe.to_string_lossy().into_owned();

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (clash, _) = hkcu.create_subkey("Software\\Classes\\Clash")?;
    clash.set_value("", &"Clash Verge Optimized")?;
    clash.set_value("URL Protocol", &"Clash Verge Optimized URL Scheme Protocol")?;
    let (default_icon, _) = hkcu.create_subkey("Software\\Classes\\Clash\\DefaultIcon")?;
    default_icon.set_value("", &app_exe)?;
    let (command, _) = hkcu.create_subkey("Software\\Classes\\Clash\\Shell\\Open\\Command")?;
    command.set_value("", &format!("{app_exe} \"%1\""))?;

    Ok(())
}

pub async fn startup_script() -> Result<()> {
    let script_path = {
        let verge = Config::verge().await;
        let verge = verge.data_arc();
        verge.startup_script.clone().unwrap_or_else(|| "".into())
    };

    if script_path.is_empty() {
        return Ok(());
    }

    let script_dir = PathBuf::from(script_path.as_str());
    let shell_type = validate_startup_script_path(&script_dir)?;

    let Some(authorization) = load_startup_script_authorization().await? else {
        logging!(
            warn,
            Type::Setup,
            "Startup script skipped because it has not been locally authorized: {}",
            script_path
        );
        return Ok(());
    };

    if !startup_script_authorization_matches(&script_dir, &authorization).await? {
        logging!(
            warn,
            Type::Setup,
            "Startup script skipped because local authorization no longer matches: {}",
            script_path
        );
        return Ok(());
    }

    let parent_dir = script_dir.parent();
    let working_dir = parent_dir.unwrap_or_else(|| script_dir.as_ref());

    Command::new(shell_type)
        .current_dir(working_dir)
        .args([script_path.as_str()])
        .output()
        .await?;

    Ok(())
}

async fn handle_copy(src: &PathBuf, dest: &PathBuf, file: &str) {
    match fs::copy(src, dest).await {
        Ok(_) => {
            logging!(debug, Type::Setup, "resources copied '{}'", file);
        }
        Err(err) => {
            logging!(
                error,
                Type::Setup,
                "failed to copy resources '{}' to '{:?}', {}",
                file,
                dest,
                err
            );
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn startup_script_authorization_requires_same_path_and_hash() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let script = temp.path().join("startup.ps1");
        fs::write(&script, "Write-Output safe").await.expect("write script");

        let authorization = build_startup_script_authorization(&script)
            .await
            .expect("build authorization");

        assert!(
            startup_script_authorization_matches(&script, &authorization)
                .await
                .expect("check authorized script")
        );

        fs::write(&script, "Write-Output changed").await.expect("modify script");
        assert!(
            !startup_script_authorization_matches(&script, &authorization)
                .await
                .expect("check changed script")
        );

        let other_script = temp.path().join("other.ps1");
        fs::write(&other_script, "Write-Output safe")
            .await
            .expect("write other script");
        assert!(
            !startup_script_authorization_matches(&other_script, &authorization)
                .await
                .expect("check other script")
        );
    }
}
