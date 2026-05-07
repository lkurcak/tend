use crate::{Job, job::filter::Filter, tui};

pub async fn run(
    job_filter: Filter,
    _verbose: bool,
    auto_start: bool,
    log_retention_days: Option<u64>,
) -> anyhow::Result<()> {
    let mut jobs = Vec::new();

    Job::iterate_jobs_filtered(
        |job| {
            jobs.push(job);
        },
        &job_filter,
        false,
        false,
    )?;

    if jobs.is_empty() {
        anyhow::bail!("No jobs matched.");
    }

    tui::run_tui(jobs, auto_start, log_retention_days).await
}
