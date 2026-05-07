use super::{
    AsyncBufReadExt, BufReader, ChildStderr, ChildStdout, ControlFlow, Job, Lines, Receiver, Result,
};

use crate::job::log::{ExitStatus, MetaEventKind, RunLogger};
use crate::tui::app::{JobEvent, JobMessage};
use process_wrap::tokio::{ChildWrapper, CommandWrap};
use tokio::sync::mpsc::Sender;

impl Job {
    async fn send_event(&self, run_id: u64, tx: &Sender<JobMessage>, event: JobEvent) {
        let _ = tx
            .send(JobMessage {
                job_name: self.name.clone(),
                run_id,
                event,
            })
            .await;
    }

    fn duration(start_time: std::time::Instant) -> std::time::Duration {
        let end_time = std::time::Instant::now();
        end_time.duration_since(start_time)
    }

    #[allow(clippy::too_many_arguments)]
    async fn wait_for_something<'a>(
        &'a self,
        run_id: u64,
        process: &mut Box<dyn ChildWrapper>,
        rx: &mut Receiver<()>,
        event_tx: &Sender<JobMessage>,
        stdout: &mut Lines<BufReader<ChildStdout>>,
        stderr: &mut Lines<BufReader<ChildStderr>>,
        start_time: std::time::Instant,
        logger: &mut RunLogger,
    ) -> Result<ControlFlow<'a>> {
        tokio::select! {
            stdout_line = stdout.next_line() => {
                if let Some(line) = stdout_line? {
                    let _ = logger.write_line(&line);
                    self.send_event(run_id, event_tx, JobEvent::StdoutLine(line.clone())).await;
                    let cf = self.stdout_line_callback(&line);
                    if let ControlFlow::RestartCommand(name)
                        | ControlFlow::FastRestartCommand(name)
                        | ControlFlow::StopJob(name) = &cf
                    {
                        let _ = logger.write_meta(MetaEventKind::HookTriggered {
                            name: (*name).to_string(),
                        });
                        self.send_event(run_id, event_tx, JobEvent::HookTriggered {
                            hook_name: (*name).to_string(),
                        }).await;
                    }
                    return Ok(cf);
                }
                Ok(ControlFlow::Nothing)
            }
            stderr_line = stderr.next_line() => {
                if let Some(line) = stderr_line? {
                    let _ = logger.write_line(&line);
                    self.send_event(run_id, event_tx, JobEvent::StderrLine(line.clone())).await;
                    let cf = self.stderr_line_callback(&line);
                    if let ControlFlow::RestartCommand(name)
                        | ControlFlow::FastRestartCommand(name)
                        | ControlFlow::StopJob(name) = &cf
                    {
                        let _ = logger.write_meta(MetaEventKind::HookTriggered {
                            name: (*name).to_string(),
                        });
                        self.send_event(run_id, event_tx, JobEvent::HookTriggered {
                            hook_name: (*name).to_string(),
                        }).await;
                    }
                    return Ok(cf);
                }
                Ok(ControlFlow::Nothing)
            }
            a = process.wait() => {
                if let Ok(status) = a
                    && status.success() {
                        let duration = Self::duration(start_time);
                        let _ = logger.write_meta(MetaEventKind::Exited {
                            status: ExitStatus::Success,
                            duration_secs: Some(duration.as_secs()),
                        });
                        let _ = logger.flush();
                        self.send_event(run_id, event_tx, JobEvent::ProcessSuccess { duration }).await;
                        return if self.restart_on_success() {
                            Ok(ControlFlow::RestartCommand("success"))
                        } else {
                            Ok(ControlFlow::StopJob("success"))
                        };
                    }

                let _ = logger.write_meta(MetaEventKind::Exited {
                    status: ExitStatus::Failure,
                    duration_secs: Some(Self::duration(start_time).as_secs()),
                });
                let _ = logger.flush();
                self.send_event(run_id, event_tx, JobEvent::ProcessFailure).await;
                if self.restart_on_failure() {
                    Ok(ControlFlow::RestartCommand("failure"))
                } else {
                    Ok(ControlFlow::StopJob("failure"))
                }
            }
            _ = rx.recv() => {
                Ok(ControlFlow::StopJob("termination signal"))
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    pub async fn create_repeated_process(
        self,
        mut rx: Receiver<()>,
        event_tx: Sender<JobMessage>,
        run_id: u64,
    ) -> Result<()> {
        let mut backoff_restart_count = 0;
        let mut fast_backoff_restart_count = 0;

        'job: loop {
            // Create a new logger for each process run
            let mut logger = match RunLogger::new(&self.name, run_id) {
                Ok(l) => l,
                Err(e) => {
                    self.send_event(
                        run_id,
                        &event_tx,
                        JobEvent::SpawnError(format!("Failed to create log file: {e}")),
                    )
                    .await;
                    break 'job;
                }
            };

            let mut command = CommandWrap::with_new(&self.program, |command| {
                command
                    .current_dir(&self.working_directory)
                    .args(&self.args)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());
            });
            #[cfg(windows)]
            {
                command.wrap(process_wrap::tokio::JobObject);
            }

            let process_result = command.spawn();
            let mut process = match process_result {
                Ok(p) => p,
                Err(e) => {
                    let _ = logger.write_meta(MetaEventKind::SpawnError {
                        error: e.to_string(),
                    });
                    self.send_event(run_id, &event_tx, JobEvent::SpawnError(e.to_string()))
                        .await;
                    break 'job;
                }
            };

            let _ = logger.write_meta(MetaEventKind::Started);
            self.send_event(run_id, &event_tx, JobEvent::Started).await;
            let start_time = std::time::Instant::now();

            let mut stdout = BufReader::new(
                process
                    .stdout()
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("Could not get stdout"))?,
            )
            .lines();

            let mut stderr = BufReader::new(
                process
                    .stderr()
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("Could not get stderr"))?,
            )
            .lines();

            loop {
                let control = self
                    .wait_for_something(
                        run_id,
                        &mut process,
                        &mut rx,
                        &event_tx,
                        &mut stdout,
                        &mut stderr,
                        start_time,
                        &mut logger,
                    )
                    .await?;

                if control == ControlFlow::Nothing {
                    continue;
                }

                // Recovery mechanism: reduce backoff count based on how long the job ran successfully
                let run_duration = Self::duration(start_time);
                let recovery_threshold = std::time::Duration::from_mins(1);

                if run_duration >= recovery_threshold {
                    fast_backoff_restart_count = 0;

                    // For every minute of successful runtime, reduce backoff count by 1
                    let recovery_amount = (run_duration.as_secs() / 60).min(backoff_restart_count);
                    backoff_restart_count = backoff_restart_count.saturating_sub(recovery_amount);
                }

                match control {
                    ControlFlow::Nothing => (),
                    ControlFlow::RestartCommand(reason) => {
                        let delay_seconds =
                            self.restart_strategy.delay_seconds(backoff_restart_count);
                        let _ = logger.write_meta(MetaEventKind::Restarting {
                            reason: reason.to_string(),
                            attempt: backoff_restart_count + 1,
                            delay_secs: delay_seconds,
                        });
                        self.send_event(
                            run_id,
                            &event_tx,
                            JobEvent::Restarting {
                                reason: reason.to_string(),
                                attempt: backoff_restart_count + 1,
                                delay_seconds,
                            },
                        )
                        .await;
                        if self
                            .sleep_or_cancel_restart(
                                delay_seconds,
                                &mut process,
                                &mut rx,
                                &event_tx,
                                run_id,
                            )
                            .await?
                        {
                            break 'job;
                        }
                        self.terminate_process(&mut process).await?;

                        backoff_restart_count += 1;

                        continue 'job;
                    }
                    ControlFlow::FastRestartCommand(reason) => {
                        let delay_seconds = self
                            .restart_strategy
                            .delay_seconds_fast(fast_backoff_restart_count);
                        let _ = logger.write_meta(MetaEventKind::Restarting {
                            reason: reason.to_string(),
                            attempt: fast_backoff_restart_count + 1,
                            delay_secs: delay_seconds,
                        });
                        self.send_event(
                            run_id,
                            &event_tx,
                            JobEvent::Restarting {
                                reason: reason.to_string(),
                                attempt: fast_backoff_restart_count + 1,
                                delay_seconds,
                            },
                        )
                        .await;
                        if self
                            .sleep_or_cancel_restart(
                                delay_seconds,
                                &mut process,
                                &mut rx,
                                &event_tx,
                                run_id,
                            )
                            .await?
                        {
                            break 'job;
                        }
                        self.terminate_process(&mut process).await?;

                        fast_backoff_restart_count += 1;

                        continue 'job;
                    }
                    ControlFlow::StopJob(reason) => {
                        let _ = logger.write_meta(MetaEventKind::Stopped {
                            reason: reason.to_string(),
                        });
                        self.send_event(
                            run_id,
                            &event_tx,
                            JobEvent::Stopped {
                                reason: reason.to_string(),
                            },
                        )
                        .await;
                        self.terminate_process(&mut process).await?;
                        break 'job;
                    }
                }
            }
        }

        self.send_event(run_id, &event_tx, JobEvent::Finished).await;
        Ok(())
    }

    async fn sleep_or_cancel_restart(
        &self,
        delay_seconds: u64,
        process: &mut Box<dyn ChildWrapper>,
        rx: &mut Receiver<()>,
        event_tx: &Sender<JobMessage>,
        run_id: u64,
    ) -> Result<bool> {
        if delay_seconds == 0 {
            return Ok(false);
        }

        let delay = tokio::time::sleep(tokio::time::Duration::from_secs(delay_seconds));
        tokio::pin!(delay);

        tokio::select! {
            () = &mut delay => Ok(false),
            _ = rx.recv() => {
                self.send_event(
                    run_id,
                    event_tx,
                    JobEvent::Stopped {
                        reason: "termination signal".to_string(),
                    },
                ).await;
                self.terminate_process(process).await?;
                Ok(true)
            }
        }
    }

    async fn terminate_process(&self, process: &mut Box<dyn ChildWrapper>) -> Result<()> {
        if let Err(e) = process.start_kill() {
            // Ignore errors from killing already-dead processes
            let _ = e;
        }

        process.wait().await?;

        Ok(())
    }
}
