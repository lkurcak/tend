use std::ffi::OsStr;

use windows_service::{
    service::{Service, ServiceAccess},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

fn connect_to_local(request_access: ServiceAccess) -> windows_service::Result<Service> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;
    let service = service_manager.open_service(crate::SERVICE_NAME, request_access)?;
    Ok(service)
}

pub fn start() -> windows_service::Result<()> {
    let service = connect_to_local(ServiceAccess::START)?;
    service.start(&[OsStr::new("")])?;
    Ok(())
}

pub fn stop() -> windows_service::Result<()> {
    let service = connect_to_local(ServiceAccess::STOP)?;
    let state = service.stop()?;
    println!("{:#?}", state);
    Ok(())
}

pub fn show_status() -> windows_service::Result<()> {
    let service = connect_to_local(ServiceAccess::QUERY_STATUS)?;
    let status = service.query_status()?;
    println!("{:#?}", status);
    Ok(())
}

pub fn show_config() -> windows_service::Result<()> {
    let service = connect_to_local(ServiceAccess::QUERY_CONFIG)?;
    let config = service.query_config()?;
    println!("{:#?}", config);
    Ok(())
}
