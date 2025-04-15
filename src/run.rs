use crate::{Job, job::filter::Filter};
use tokio::sync::mpsc;

pub async fn run(job_filter: Filter, verbose: bool) -> anyhow::Result<()> {
    let mut join_set = tokio::task::JoinSet::new();
    let mut cancel_handles = vec![];

    let mut count = 0;

    Job::iterate_jobs_filtered(
        |job| {
            count += 1;

            let (tx, rx) = mpsc::channel::<()>(1);
            cancel_handles.push(tx);
            join_set.spawn(job.create_repeated_process(rx, verbose));
        },
        &job_filter,
        false,
        verbose,
    )?;

    if count == 0 {
        anyhow::bail!("No jobs matched.");
    }

    loop {
        tokio::select! {
            a = join_set.join_next() => {
                if a.is_none() {
                    if verbose {
                        println!("All jobs finished.");
                    }
                    break;
                }
            }

            _ = tokio::signal::ctrl_c() => {
                for tx in &cancel_handles {
                    let _ = tx.send(()).await;
                }

                join_set.shutdown().await;
            }
        }
    }

    Ok(())
}
