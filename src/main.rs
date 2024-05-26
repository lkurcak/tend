use std::collections::HashMap;

use crate::colors::TendColors;
mod args;
mod colors;
mod job;
mod run;

use crate::job::{Job, JobFilter};
use anyhow::Result;
use clap::Parser;

fn standard_job_filter(
    name: Option<String>,
    _all: bool,
    group: Vec<String>,
    job: Vec<String>,
    exclude: Vec<String>,
) -> JobFilter {
    if group.is_empty() && job.is_empty() {
        if let Some(name) = name {
            JobFilter::Subset {
                groups: vec![],
                jobs: vec![name],
                exclude,
            }
        } else {
            JobFilter::All { exclude }
        }
    } else {
        JobFilter::Subset {
            groups: group,
            jobs: job,
            exclude,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = args::Cli::parse();

    if args.no_color {
        colored::control::set_override(false);
    }

    match args.command {
        args::Commands::List {
            all,
            group,
            job,
            exclude,
            name,
        } => {
            let filter = standard_job_filter(name, all, group, job, exclude);

            Job::list(filter)?;
        }
        args::Commands::Run {
            name,
            group,
            job,
            all,
            exclude,
        } => {
            let filter = standard_job_filter(name, all, group, job, exclude);

            run::run(filter, args.verbose).await?;
        }
        args::Commands::Create {
            name,
            program,
            args,
            restart,
            group,
            overwrite,
            restart_strategy,
        } => {
            let job = Job {
                name,
                program,
                args,
                restart,
                group,
                working_directory: std::env::current_dir()?,
                restart_strategy,
                event_hooks: HashMap::new(),
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
        args::Commands::Edit { name, command } => {
            let mut job = Job::load(&name)?;
            match command {
                args::EditJobCommands::Group { group } => job.group = group,
                args::EditJobCommands::Hook { command } => match command {
                    args::EditJobHookCommands::List => {
                        if job.event_hooks.is_empty() {
                            println!("No hooks defined for job {}", job.name);
                        } else {
                            for (name, hook) in job.event_hooks.iter() {
                                println!("{}: {:?}", name, hook);
                            }
                        }
                    }
                    args::EditJobHookCommands::Create { hook, t } => match t {
                        args::JobHook::DetectedSubstring { substring, stream } => {
                            job.event_hooks.insert(
                                hook.clone(),
                                job::JobEventHook {
                                    event: job::JobEvent::DetectedSubstring {
                                        contains: substring,
                                        stream,
                                    },
                                    action: job::JobAction::Restart,
                                },
                            );
                        }
                    },
                    args::EditJobHookCommands::Delete { hook } => {
                        match job.event_hooks.remove(&hook) {
                            Some(_) => println!("Hook {} deleted", hook),
                            None => eprintln!("Hook {} not found", hook),
                        }
                    }
                },
            }
            job.save(true)?;
        }
        args::Commands::Delete {
            name,
            group,
            all,
            confirm,
            job,
            exclude,
        } => {
            let filter = standard_job_filter(name, all, group, job, exclude);

            if all && !confirm {
                eprintln!(
                    "{}",
                    "Use --confirm to delete all jobs. This cannot be undone.".failure()
                );
            } else {
                Job::iterate_jobs_filtered(
                    |job| {
                        let _ = job.delete();
                    },
                    &filter,
                )?;
            }
        }
    }

    Ok(())
}
