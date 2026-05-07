pub mod app;
pub mod ui;

use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::{sync::mpsc, task::JoinSet};

use crate::job::Job;
use crate::job::log::cleanup_retention;

use self::app::{App, AppCommand, JobEvent, JobMessage};

const EVENT_CHANNEL_CAPACITY: usize = 4096;
const DEFAULT_LOG_RETENTION_DAYS: u64 = 7;
const RETENTION_CLEANUP_INTERVAL_SECS: u64 = 3600; // 1 hour

type JobTaskResult = (String, u64, Result<()>);

#[derive(Debug)]
struct TerminalGuard {
    raw_mode: bool,
    alternate_screen: bool,
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut guard = Self {
            raw_mode: true,
            alternate_screen: false,
        };

        let mut stdout = std::io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = guard.restore();
            return Err(error.into());
        }

        guard.alternate_screen = true;
        Ok(guard)
    }

    fn restore(&mut self) -> Result<()> {
        if self.alternate_screen {
            let mut stdout = std::io::stdout();
            execute!(stdout, LeaveAlternateScreen)?;
            self.alternate_screen = false;
        }

        if self.raw_mode {
            disable_raw_mode()?;
            self.raw_mode = false;
        }

        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

pub async fn run_tui(
    jobs: Vec<Job>,
    auto_start: bool,
    log_retention_days: Option<u64>,
) -> Result<()> {
    let retention_days = log_retention_days.unwrap_or(DEFAULT_LOG_RETENTION_DAYS);

    // Run retention cleanup on startup (in background to not block TUI launch)
    tokio::task::spawn_blocking(move || cleanup_retention(retention_days));

    // Spawn periodic retention cleanup task
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_secs(RETENTION_CLEANUP_INTERVAL_SECS));
        interval.tick().await; // Skip the first immediate tick (already ran on startup)
        loop {
            interval.tick().await;
            let days = retention_days;
            tokio::task::spawn_blocking(move || cleanup_retention(days));
        }
    });

    let (event_tx, event_rx) = mpsc::channel::<JobMessage>(EVENT_CHANNEL_CAPACITY);
    let mut join_set = JoinSet::new();
    let mut app = App::new(jobs, event_rx);

    let mut terminal_guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    if auto_start {
        start_all_jobs(&mut app, &event_tx, &mut join_set);
    }

    let result = run_event_loop(&mut terminal, &mut app, &event_tx, &mut join_set).await;

    for job_info in &mut app.jobs {
        if let Some(tx) = job_info.cancel_tx.take() {
            let _ = tx.send(()).await;
        }
    }

    let shutdown_timeout = tokio::time::sleep(Duration::from_secs(3));
    tokio::pin!(shutdown_timeout);
    loop {
        drain_job_events(&mut app);
        tokio::select! {
            result = join_set.join_next() => {
                if result.is_none() {
                    break;
                }
            }
            () = &mut shutdown_timeout => {
                join_set.shutdown().await;
                break;
            }
        }
    }

    let cleanup_result = terminal
        .show_cursor()
        .map_err(anyhow::Error::from)
        .and_then(|()| terminal_guard.restore());

    match (result, cleanup_result) {
        (Err(error), _) | (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    event_tx: &mpsc::Sender<JobMessage>,
    join_set: &mut JoinSet<JobTaskResult>,
) -> Result<()> {
    loop {
        drain_job_events(app);
        drain_finished_tasks(app, join_set);

        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(16))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            let commands = app.handle_key_event(key);
            for cmd in commands {
                execute_command(app, cmd, event_tx, join_set).await;
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn drain_job_events(app: &mut App) {
    while let Ok(msg) = app.event_rx.try_recv() {
        app.handle_job_event(msg);
    }
}

fn drain_finished_tasks(app: &mut App, join_set: &mut JoinSet<JobTaskResult>) {
    while let Some(result) = join_set.try_join_next() {
        if let Ok((job_name, run_id, Err(error))) = result {
            app.handle_job_event(JobMessage {
                job_name,
                run_id,
                event: JobEvent::TaskError(error.to_string()),
            });
        }
    }
}

async fn execute_command(
    app: &mut App,
    command: AppCommand,
    event_tx: &mpsc::Sender<JobMessage>,
    join_set: &mut JoinSet<JobTaskResult>,
) {
    match command {
        AppCommand::StartJob(idx) => start_job(app, idx, event_tx, join_set),
        AppCommand::StopJob(idx) => stop_job(app, idx).await,
        AppCommand::RestartJob(idx) => restart_job(app, idx, event_tx, join_set).await,
        AppCommand::OpenLogs(idx) => open_logs(app, idx),
        AppCommand::Quit => {
            app.should_quit = true;
        }
    }
}

fn start_all_jobs(
    app: &mut App,
    event_tx: &mpsc::Sender<JobMessage>,
    join_set: &mut JoinSet<JobTaskResult>,
) {
    for idx in 0..app.jobs.len() {
        start_job(app, idx, event_tx, join_set);
    }
}

fn start_job(
    app: &mut App,
    idx: usize,
    event_tx: &mpsc::Sender<JobMessage>,
    join_set: &mut JoinSet<JobTaskResult>,
) {
    if app
        .jobs
        .get(idx)
        .is_some_and(|job_info| job_info.status != app::JobStatus::Stopped)
    {
        return;
    }

    start_job_unchecked(app, idx, event_tx, join_set);
}

fn start_job_unchecked(
    app: &mut App,
    idx: usize,
    event_tx: &mpsc::Sender<JobMessage>,
    join_set: &mut JoinSet<JobTaskResult>,
) {
    let Some(job) = app.jobs.get(idx).map(|job_info| job_info.job.clone()) else {
        return;
    };

    let run_id = app.next_run_id();
    let (cancel_tx, cancel_rx) = mpsc::channel::<()>(1);
    spawn_job(job, cancel_rx, event_tx.clone(), run_id, join_set);

    if let Some(job_info) = app.jobs.get_mut(idx) {
        job_info.cancel_tx = Some(cancel_tx);
        job_info.active_run_id = Some(run_id);
        job_info.status = app::JobStatus::Running;
        job_info.started_at = None;
    }
}

fn spawn_job(
    job: Job,
    cancel_rx: mpsc::Receiver<()>,
    event_tx: mpsc::Sender<JobMessage>,
    run_id: u64,
    join_set: &mut JoinSet<JobTaskResult>,
) {
    let job_name = job.name.clone();
    join_set.spawn(async move {
        let result = job
            .create_repeated_process(cancel_rx, event_tx, run_id)
            .await;
        (job_name, run_id, result)
    });
}

async fn stop_job(app: &mut App, idx: usize) {
    if let Some(job_info) = app.jobs.get_mut(idx)
        && let Some(tx) = job_info.cancel_tx.take()
    {
        let _ = tx.send(()).await;
    }
}

async fn restart_job(
    app: &mut App,
    idx: usize,
    event_tx: &mpsc::Sender<JobMessage>,
    join_set: &mut JoinSet<JobTaskResult>,
) {
    stop_job(app, idx).await;
    start_job_unchecked(app, idx, event_tx, join_set);
}

fn open_logs(app: &App, idx: usize) {
    let Some(job_info) = app.jobs.get(idx) else {
        return;
    };

    let Ok(log_dir) = crate::job::log::job_log_dir(&job_info.job.name) else {
        return;
    };

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(&log_dir).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&log_dir).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&log_dir)
            .spawn();
    }
}
