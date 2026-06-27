/// stop and uninstall the service
fn main() -> anyhow::Result<()> {
    use platform_lib::{
        service::{ServiceAccess, ServiceState},
        service_manager::{ServiceManager, ServiceManagerAccess},
    };
    use std::{thread, time::Duration};

    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service("clash_verge_service", service_access)?;

    let service_status = service.query_status()?;
    if service_status.current_state != ServiceState::Stopped {
        if let Err(err) = service.stop() {
            eprintln!("{err}");
        }
        // Wait for service to stop
        thread::sleep(Duration::from_secs(1));
    }

    service.delete()?;
    println!("Service uninstalled successfully. Resource cleanup warnings can be ignored.");
    Ok(())
}
