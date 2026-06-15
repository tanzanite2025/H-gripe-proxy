use anyhow::Error;
use std::path::Path;

use super::macos_service_identity as identity;

type CommandRunner = fn(&str, &[&str], bool) -> Result<(), Error>;

// Legacy migration from Clash Verge Rev.
// Keep these cleanup steps until the previous privileged helper and bundle-based
// service are no longer part of a supported upgrade path.
pub fn cleanup_legacy_services(debug: bool, run_command: CommandRunner) -> Result<(), Error> {
    let helper_result = cleanup_legacy_helper_service(debug, run_command);
    let bundle_result = cleanup_legacy_bundle_service(debug, run_command);

    helper_result?;
    bundle_result?;
    Ok(())
}

fn cleanup_legacy_helper_service(debug: bool, run_command: CommandRunner) -> Result<(), Error> {
    let target_binary_path = identity::legacy_helper_binary_path();
    let plist_file = identity::legacy_helper_plist_path();
    let system_target = identity::legacy_helper_system_target();

    let _ = run_command("launchctl", &["stop", identity::LEGACY_HELPER_ID], debug);
    let _ = run_command("launchctl", &["bootout", "system", plist_file.as_str()], debug);
    let _ = run_command("launchctl", &["disable", system_target.as_str()], debug);

    remove_file_if_exists(plist_file.as_str(), "legacy helper plist")?;
    remove_file_if_exists(target_binary_path.as_str(), "legacy helper binary")?;

    Ok(())
}

fn cleanup_legacy_bundle_service(debug: bool, run_command: CommandRunner) -> Result<(), Error> {
    let bundle_path = identity::legacy_service_bundle_path();
    let plist_file = identity::legacy_service_plist_path();
    let system_target = identity::legacy_launchctl_system_target();

    let _ = run_command("launchctl", &["stop", identity::LEGACY_SERVICE_BUNDLE_ID], debug);
    let _ = run_command("launchctl", &["disable", system_target.as_str()], debug);
    let _ = run_command("launchctl", &["bootout", "system", plist_file.as_str()], debug);

    remove_file_if_exists(plist_file.as_str(), "legacy bundle plist")?;
    remove_dir_if_exists(bundle_path.as_str(), "legacy bundle directory")?;

    Ok(())
}

fn remove_file_if_exists(path: &str, label: &str) -> Result<(), Error> {
    if Path::new(path).exists() {
        std::fs::remove_file(path).map_err(|e| anyhow::anyhow!("Failed to remove {label}: {}", e))?;
    }

    Ok(())
}

fn remove_dir_if_exists(path: &str, label: &str) -> Result<(), Error> {
    if Path::new(path).exists() {
        std::fs::remove_dir_all(path).map_err(|e| anyhow::anyhow!("Failed to remove {label}: {}", e))?;
    }

    Ok(())
}
