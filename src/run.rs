use crate::{job::JobFilter, Job};
use std::collections::HashMap;
use tokio::sync::mpsc;

pub async fn run(job_filter: JobFilter, verbose: bool) -> anyhow::Result<()> {
    let mut join_handles = HashMap::new();
    let mut cancel_handles = HashMap::new();

    let mut count = 0;

    Job::iterate_jobs_filtered(
        |job| {
            count += 1;

            let (tx, rx) = mpsc::channel::<()>(1);
            let handle = tokio::spawn(job.clone().create_repeated_process(rx, verbose));
            cancel_handles.insert(job.name.clone(), tx);
            join_handles.insert(job.name.clone(), handle);
        },
        &job_filter,
    )?;

    if count == 0 {
        anyhow::bail!("No jobs matched.");
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
