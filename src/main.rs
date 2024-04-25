mod install_service;
mod notify_service;
mod service;
mod uninstall_service;

use anyhow::Result;
use clap::{Parser, Subcommand};

const SERVICE_NAME: &str = "ping_service";

#[derive(Debug, Parser)]
#[command(name = "git")]
#[command(about = "A fictional versioning CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(subcommand, about = "Manage the background service")]
    Service(ServiceCommands),
}

#[derive(Debug, Subcommand)]
enum ServiceCommands {
    #[command(about = "Install the service")]
    Install,
    #[command(about = "Start the service")]
    Start,
    #[command(about = "Show service status")]
    Status,
    #[command(about = "Show service configuration")]
    Config,
    #[command(about = "Stop the service")]
    Stop,
    #[command(about = "Uninstall the service")]
    Uninstall,
}

#[cfg(windows)]
fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Service(command) => match command {
            ServiceCommands::Install => install_service::main()?,
            ServiceCommands::Start => service::start()?,
            ServiceCommands::Status => service::show_status()?,
            ServiceCommands::Config => service::show_config()?,
            ServiceCommands::Stop => service::stop()?,
            ServiceCommands::Uninstall => uninstall_service::main()?,
        },
    }

    Ok(())
}
