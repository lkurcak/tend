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
            template,
        } => {
            let mut job = Job {
                name,
                program,
                args,
                restart,
                group,
                working_directory: std::env::current_dir()?,
                restart_strategy,
                event_hooks: HashMap::new(),
                template,
            };

            if let Some(template) = template {
                match template {
                    crate::job::JobTemplate::PortForward => {
                        // @note: Examples of errors from `kubectl port-forward`:
                        // E0515 11:45:17.837897   23508 portforward.go:372] error copying from remote stream to local connection: readfrom tcp4 127.0.0.1:8443->127.0.0.1:50656: write tcp4 127.0.0.1:8443->127.0.0.1:50656: wsasend: An established connection was aborted by the software in your host machine.
                        // E0515 11:45:51.137714   23508 portforward.go:340] error creating error stream for port 8443 -> 8443: Timeout occurred
                        // E0515 11:45:51.293626   23508 portforward.go:362] error creating forwarding stream for port 8443 -> 8443: Timeout occurred
                        // E0515 11:45:52.013842   23508 portforward.go:362] error creating forwarding stream for port 8443 -> 8443: Timeout occurred
                        // E0515 11:46:53.524413   23508 portforward.go:400] an error occurred forwarding 8443 -> 8443: error forwarding port 8443 to pod 20919150d2fddf20d4b94e389744ffde70ae784debf216326d58c7dd0d79401e, uid : failed to execute portforward in network namespace "/var/run/netns/cni-d0e0bbba-6286-aba6-45a1-fa24e0e614e2": failed to connect to localhost:8443 inside namespace "20919150d2fddf20d4b94e389744ffde70ae784debf216326d58c7dd0d79401e", IPv4: dial tcp4 127.0.0.1:8443: connect: connection refused IPv6 dial tcp6: address localhost: no suitable address found
                        // E0515 11:46:53.608922   23508 portforward.go:400] an error occurred forwarding 8443 -> 8443: error forwarding port 8443 to pod 20919150d2fddf20d4b94e389744ffde70ae784debf216326d58c7dd0d79401e, uid : failed to execute portforward in network namespace "/var/run/netns/cni-d0e0bbba-6286-aba6-45a1-fa24e0e614e2": failed to connect to localhost:8443 inside namespace "20919150d2fddf20d4b94e389744ffde70ae784debf216326d58c7dd0d79401e", IPv4: dial tcp4 127.0.0.1:8443: connect: connection refused IPv6 dial tcp6: address localhost: no suitable address found
                        // E0515 11:47:03.229256   23508 portforward.go:340] error creating error stream for port 8443 -> 8443: Timeout occurred
                        // E0515 11:47:07.613031   23508 portforward.go:340] error creating error stream for port 8443 -> 8443: Timeout occurred
                        // E0515 11:47:23.206203   23508 portforward.go:340] error creating error stream for port 8443 -> 8443: Timeout occurred
                        // E0515 11:47:23.409211   23508 portforward.go:362] error creating forwarding stream for port 8443 -> 8443: Timeout occurred
                        // E0515 11:47:23.786188   23508 portforward.go:400] an error occurred forwarding 8443 -> 8443: error forwarding port 8443 to pod 20919150d2fddf20d4b94e389744ffde70ae784debf216326d58c7dd0d79401e, uid : network namespace for sandbox "20919150d2fddf20d4b94e389744ffde70ae784debf216326d58c7dd0d79401e" is closed
                        // E0515 11:47:23.973797   23508 portforward.go:362] error creating forwarding stream for port 8443 -> 8443: Timeout occurred
                        // E0515 11:47:24.066781   23508 portforward.go:362] error creating forwarding stream for port 8443 -> 8443: Timeout occurred
                        // E0515 11:47:37.957485   23508 portforward.go:340] error creating error stream for port 8443 -> 8443: Timeout occurred
                        job.event_hooks.insert(
                            "pfw-template-hook-1".to_string(),
                            job::JobEventHook {
                                event: job::JobEvent::DetectedSubstring {
                                    contains: "error".to_string(),
                                    stream: job::Stream::Any,
                                },
                                action: job::JobAction::Restart,
                            },
                        );
                        job.event_hooks.insert(
                            "pfw-template-hook-2".to_string(),
                            job::JobEventHook {
                                event: job::JobEvent::DetectedSubstring {
                                    contains: "aborted".to_string(),
                                    stream: job::Stream::Any,
                                },
                                action: job::JobAction::Restart,
                            },
                        );
                    }
                }
            }

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
                        args::JobHook::DetectedSubstring {
                            substring,
                            stream,
                            action,
                        } => {
                            job.event_hooks.insert(
                                hook.clone(),
                                job::JobEventHook {
                                    event: job::JobEvent::DetectedSubstring {
                                        contains: substring,
                                        stream,
                                    },
                                    action,
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
