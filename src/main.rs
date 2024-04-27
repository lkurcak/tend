mod colors;
mod install_service;
mod job;
mod ping_service;
mod service;
mod uninstall_service;

use crate::job::Job;
use crate::job::JobStatus;
use anyhow::Result;
use clap::{Parser, Subcommand};

const SERVICE_NAME: &str = "ping_service";
const DEFAULT_GROUP: &str = "default";

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
    #[command(about = "List jobs")]
    List {
        #[arg(long, short = 'g', help = "Filter by group")]
        group: Option<String>,
    },
    #[command(about = "Start a job")]
    Start { name: String },
    #[command(about = "Create a job")]
    Create {
        name: String,
        // #[arg(long, short = 'd', help = "Job description")]
        // description: Option<String>,
        program: String,
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
        #[arg(long, short = 'g', help = "Add job to a group", default_value = DEFAULT_GROUP)]
        group: String,
        args: Vec<String>,
    },
    #[command(about = "Delete a job")]
    Delete { name: String },
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
    #[command(hide = true, about = "Run the background service")]
    Run,
    #[command(hide = false, about = "Run the service in the foreground")]
    RunForeground,
}

#[cfg(windows)]
#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Service(command) => match command {
            ServiceCommands::Install => install_service::main()?,
            ServiceCommands::Start => service::start()?,
            ServiceCommands::Status => service::show_status()?,
            ServiceCommands::Run => ping_service::main()?,
            ServiceCommands::RunForeground => ping_service::run_service::<true>().await?,
            ServiceCommands::Config => service::show_config()?,
            ServiceCommands::Stop => service::stop()?,
            ServiceCommands::Uninstall => uninstall_service::main()?,
        },
        Commands::Job(command) => match command {
            JobCommands::List { group } => {
                Job::list(group)?;
            }
            JobCommands::Start { name } => {
                let job = Job::load(&name)?;
                // job.run_once().await?;
                todo!()
            }
            JobCommands::Create {
                name,
                program,
                args,
                restart_on_failure,
                restart_on_success,
                start_immediately,
                group,
            } => {
                let job = Job {
                    name,
                    program,
                    args,
                    restart_on_failure,
                    restart_on_success,
                    group,
                    working_directory: std::env::current_dir()?,
                    restart_requested: start_immediately,
                    status: JobStatus::Stopped,
                };
                job.save()?;
                if start_immediately {
                    // job.run_once().await?;
                    todo!()
                }
            }
            JobCommands::Delete { name } => {
                let job = Job::load(&name)?;
                job.delete()?;
            }
        },
    }

    Ok(())
}
