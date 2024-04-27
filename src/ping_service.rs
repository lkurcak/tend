use crate::colors::TendColors;
use crate::Job;
use crate::JobStatus;
use std::collections::HashMap;
use std::io::Write;
use std::{
    fs::File,
    io,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::process::Child;

/// Logs a message to a specified log file.
fn log_message(file: &mut File, message: &str) -> io::Result<()> {
    // Optionally, prepend a timestamp or other metadata to each log entry
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let timestamp = since_the_epoch.as_secs();

    writeln!(file, "{}: {}", timestamp, message)
}

use std::{ffi::OsString, fs::OpenOptions, time::Duration};
use tokio::sync::mpsc;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher, Result,
};

const SERVICE_NAME: &str = "ping_service";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

const LOOPBACK_ADDR: [u8; 4] = [127, 0, 0, 1];
const RECEIVER_PORT: u16 = 1234;
const PING_MESSAGE: &str = "ping from the ping service 123\n";

#[cfg(windows)]
pub fn main() -> Result<()> {
    println!("Starting ping service");
    // Register generated `ffi_service_main` with the system and start the service, blocking
    // this thread until the service is stopped.
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

// Generate the windows service boilerplate.
// The boilerplate contains the low-level service entry function (ffi_service_main) that parses
// incoming service arguments into Vec<OsString> and passes them to user defined service
// entry (my_service_main).
define_windows_service!(ffi_service_main, my_service_main);

// Service entry function which is called on background thread by the system with service
// parameters. There is no stdout or stderr at this point so make sure to configure the log
// output to file if needed.
#[tokio::main]
pub async fn my_service_main(_arguments: Vec<OsString>) {
    // Open or create a log file
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("app.log") {
        // Use the log_message function to log some messages
        let _ = log_message(&mut file, "The service is starting up");
        let _ = log_message(&mut file, "Performing an operation...");
        let _ = log_message(&mut file, "The service is shutting down");
    }

    if let Err(_e) = run_service::<false>().await {
        // Handle the error, by logging or something.
    }
}

fn do_jobs(
    join_handles: &mut HashMap<String, tokio::task::JoinHandle<()>>,
    cancel_handles: &mut HashMap<String, mpsc::Sender<()>>,
) -> anyhow::Result<()> {
    Job::iterate_jobs(|job| {
        let (tx, rx) = mpsc::channel::<()>(1);
        tokio::spawn(job.clone().create_repeated_process(rx));
        cancel_handles.insert(job.name, tx);
    })?;
    Ok(())
}

pub async fn run_service<const FOREGROUND: bool>() -> anyhow::Result<()> {
    println!("Starting ping service");
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();
    let (refresh_tx, refresh_rx) = std::sync::mpsc::channel();

    let mut handle = None;

    if !FOREGROUND {
        let event_handler = move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

                ServiceControl::Stop => {
                    shutdown_tx.send(()).unwrap();
                    ServiceControlHandlerResult::NoError
                }

                ServiceControl::UserEvent(code) => {
                    if code.to_raw() == crate::service::ServiceUserCodes::Refresh as u32 {
                        refresh_tx.send(()).unwrap();
                    }
                    ServiceControlHandlerResult::NoError
                }

                _ => ServiceControlHandlerResult::NotImplemented,
            }
        };

        let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;
        handle = Some(status_handle);

        status_handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;
    }

    let mut join_handles = HashMap::new();
    let mut cancel_handles = HashMap::new();
    do_jobs(&mut join_handles, &mut cancel_handles)?;

    // loop {
    //     match refresh_rx.recv_timeout(Duration::from_secs(1)) {
    //         Ok(_) => {
    //             // do_jobs(&mut cancel_handles)?;
    //         }
    //         Err(std::sync::mpsc::RecvTimeoutError::Timeout) => (),
    //         Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
    //     }
    //     match shutdown_rx.recv_timeout(Duration::from_secs(1)) {
    //         Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
    //         Err(std::sync::mpsc::RecvTimeoutError::Timeout) => (),
    //     };
    // }

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("Ctrl-C received, shutting down");
        }
    }

    for (name, tx) in cancel_handles {
        println!("Sending cancel signal to {}", name.job());
        tx.send(()).await?;
    }

    for (name, handle) in join_handles {
        println!("Waiting for {} to finish", name.job());
        handle.await?;
    }

    if !FOREGROUND {
        if let Some(status_handle) = handle {
            // Tell the system that service has stopped.
            status_handle.set_service_status(ServiceStatus {
                service_type: SERVICE_TYPE,
                current_state: ServiceState::Stopped,
                controls_accepted: ServiceControlAccept::empty(),
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: Duration::default(),
                process_id: None,
            })?;
        }
    }

    Ok(())
}
