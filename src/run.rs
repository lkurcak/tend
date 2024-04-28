use crate::{job::JobFilter, Job};
use std::collections::HashMap;
use tokio::sync::mpsc;

pub async fn run(start_args: JobFilter) -> anyhow::Result<()> {
    let mut join_handles = HashMap::new();
    let mut cancel_handles = HashMap::new();

    let mut count = 0;

    Job::iterate_jobs(|job| {
        match &start_args {
            JobFilter::All => {}
            JobFilter::Job { job: name } => {
                if &job.name != name {
                    return;
                }
            }
            JobFilter::Group { group } => {
                if &job.group != group {
                    return;
                }
            }
        }

        count += 1;

        let (tx, rx) = mpsc::channel::<()>(1);
        let handle = tokio::spawn(job.clone().create_repeated_process(rx));
        cancel_handles.insert(job.name.clone(), tx);
        join_handles.insert(job.name.clone(), handle);
    })?;

    if count == 0 {
        println!("No jobs matched.");
        return Ok(());
    }

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            for (_name, tx) in cancel_handles {
                let _ = tx.send(()).await;
            }

            for (_name, handle) in join_handles {
                let _ = handle.await;
            }
        }
    }

    Ok(())
}
