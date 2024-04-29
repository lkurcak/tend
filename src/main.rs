use crate::colors::TendColors;
mod args;
mod colors;
mod job;
mod run;

use crate::job::{Job, JobFilter};
use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let args = args::Cli::parse();

    match args.command {
        args::Commands::List { group } => {
            Job::list(group)?;
        }
        args::Commands::Run { group, job, all: _ } => {
            let filter: JobFilter = if let Some(group) = group {
                JobFilter::Group { group }
            } else if let Some(job) = job {
                JobFilter::Job { job }
            } else {
                JobFilter::All
            };
            run::run(filter).await?;
        }
        args::Commands::Create {
            name,
            program,
            args,
            restart,
            group,
            overwrite,
        } => {
            let job = Job {
                name,
                program,
                args,
                restart,
                group,
                working_directory: std::env::current_dir()?,
            };
            let res = job.save(overwrite);
            if let Err(ref error) = res {
                if let Some(error) = error.downcast_ref::<std::io::Error>() {
                    if error.kind() == std::io::ErrorKind::AlreadyExists {
                        eprintln!(
                            "{}",
                            "Job already exists. Use --overwrite to replace it.".failure()
                        );
                        return Ok(());
                    }
                }
            }
            res?;
        }
        args::Commands::Delete { name } => {
            let job = Job::load(&name)?;
            job.delete()?;
        }
    }

    Ok(())
}
