mod install_service;
mod notify_service;
mod service;
mod uninstall_service;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

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
    #[command(subcommand, about = "Manage jobs")]
    Job(JobCommands),
    #[command(subcommand, about = "Manage the OS background service")]
    Service(ServiceCommands),
}

#[derive(Debug, Subcommand)]
enum JobCommands {
    #[command(about = "Run a job")]
    Run { name: String },
    #[command(about = "Create a job")]
    Create {
        name: String,
        command: String,
        #[arg(
            long,
            default_value = "false",
            short = 'i',
            help = "Start job immediately"
        )]
        start_immediately: bool,
        #[arg(
            long,
            default_value = "false",
            short = 'f',
            help = "Restart job on failure"
        )]
        restart_on_failure: bool,
        #[arg(
            long,
            default_value = "false",
            short = 's',
            help = "Restart job on success"
        )]
        restart_on_success: bool,
        #[arg(long, short = 'g', help = "Add job to a group")]
        group: Option<String>,
        args: Vec<String>,
    },
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
        Commands::Job(command) => match command {
            JobCommands::Run { name } => {
                let mut job = std::process::Command::new("ping")
                    .arg("localhost")
                    .spawn()?;
                println!("{:?}", job.wait()?);
            }

            JobCommands::Create {
                name,
                command,
                args,
                restart_on_failure,
                restart_on_success,
                start_immediately,
                group,
            } => {
                println!("Creating job {}", name);
                let mut job = std::process::Command::new(command).args(args).spawn()?;
                if let Ok(status) = job.wait() {
                    if status.success() {
                        println!("Job {} created successfully", name);
                    } else {
                        println!("Job {} failed", name);
                    }
                }
            }
        },
    }

    Ok(())
}
