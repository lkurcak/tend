use super::{
    AsyncBufReadExt, BufReader, ChildStderr, ChildStdout, ControlFlow, Folktime, Job, Lines,
    Receiver, Result, Tend,
};

use process_wrap::tokio::{TokioChildWrapper, TokioCommandWrap};

impl Job {
    async fn wait_for_something(
        &self,
        process: &mut Box<dyn TokioChildWrapper>,
        rx: &mut Receiver<()>,
        verbose: bool,
        stdout: &mut Lines<BufReader<ChildStdout>>,
        stderr: &mut Lines<BufReader<ChildStderr>>,
    ) -> Result<ControlFlow> {
        let process: &mut tokio::process::Child = process.inner_mut();

        tokio::select! {
            stdout_line = stdout.next_line() => {
                if let Some(line) = stdout_line? {
                    if verbose {
                        println!("{}{}{}{}", self.name.job(), " (stdout)".thick(), ": ".job(), line);
                    }else {
                        println!("{}{}", format!("{}: ", self.name).job(), line);
                    }
                    return Ok(self.stdout_line_callback(&line, verbose));
                }
                Ok(ControlFlow::Nothing)
            }
            stderr_line = stderr.next_line() => {
                if let Some(line) = stderr_line? {
                    println!("{}{}{}{}", self.name.job(), " (stderr)".failure(), ": ".job(), line);
                    return Ok(self.stderr_line_callback(&line, verbose));
                }
                Ok(ControlFlow::Nothing)
            }
            a = process.wait() => {
                if let Ok(status) = a {
                    if status.success() {
                        println!(
                            "{} process finished indicating {}",
                            self.name.job(),
                            "success".success(),
                        );
                        return if self.restart_on_success() {
                            Ok(ControlFlow::RestartCommand)
                        } else {
                            Ok(ControlFlow::StopJob)
                        };
                    }
                }

                println!(
                    "{} process finished indicating {}",
                    self.name.job(),
                    "failure".failure(),
                );
                if self.restart_on_failure() {
                    Ok(ControlFlow::RestartCommand)
                } else {
                    Ok(ControlFlow::StopJob)
                }
            }
            _ = rx.recv() => {
                if verbose {
                    println!("{} received termination signal", self.name.job());
                }
                let _ = process.kill().await;
                Ok(ControlFlow::StopJob)
            }
        }
    }

    pub async fn create_repeated_process(self, mut rx: Receiver<()>, verbose: bool) -> Result<()> {
        let mut backoff_restart_count = 0;

        'job: loop {
            let mut command = TokioCommandWrap::with_new(&self.program, |command| {
                command
                    .current_dir(&self.working_directory)
                    .args(&self.args)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());
            });
            // NOTE: this actually made the subprocesses detach when tested on Ubuntu :D
            //#[cfg(unix)]
            //{ command.wrap(process_wrap::tokio::ProcessGroup::leader()); }
            #[cfg(windows)]
            {
                command.wrap(process_wrap::tokio::JobObject);
            }

            let mut process = command.spawn()?;

            if verbose {
                println!("{} starting", self.name.job(),);
            }
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
                    .wait_for_something(&mut process, &mut rx, verbose, &mut stdout, &mut stderr)
                    .await?;

                if control == ControlFlow::Nothing {
                    continue;
                }

                let end_time = std::time::Instant::now();
                let job_duration = end_time.duration_since(start_time);
                let reset_backoff_duration = std::time::Duration::from_secs(60 * 10);
                if job_duration >= reset_backoff_duration {
                    backoff_restart_count = 0;
                }

                print!(
                    "{} ran for {}",
                    self.name.job(),
                    Folktime::duration(job_duration).to_string().time_value(),
                );

                match control {
                    ControlFlow::Nothing => (),
                    ControlFlow::RestartCommand => {
                        let delay_seconds =
                            self.restart_strategy.delay_seconds(backoff_restart_count);
                        if delay_seconds != 0 {
                            println!(
                                " (restarting in {} seconds)",
                                delay_seconds.to_string().time_value()
                            );
                            tokio::time::sleep(tokio::time::Duration::from_secs(delay_seconds))
                                .await;
                            self.terminate_process(&mut process, verbose).await?;
                        } else {
                            println!(" (restarting)");
                            self.terminate_process(&mut process, verbose).await?;
                        }

                        backoff_restart_count += 1;

                        continue 'job;
                    }
                    ControlFlow::StopJob => {
                        if verbose {
                            println!(" (stopping)");
                        } else {
                            println!();
                        }
                        self.terminate_process(&mut process, verbose).await?;
                        break 'job;
                    }
                }
            }
        }

        Ok(())
    }

    async fn terminate_process(
        &self,
        process: &mut Box<dyn TokioChildWrapper>,
        verbose: bool,
    ) -> Result<()> {
        if verbose {
            println!("{} terminating process", self.name.job());
        }

        if let Err(e) = process.start_kill() {
            eprintln!("{} failed to send SIGTERM: {}", self.name.job(), e);
        }

        if verbose {
            println!("{} waiting for process to terminate", self.name.job());
        }

        let wait_future = Box::into_pin(process.wait());
        wait_future.await?;

        Ok(())
    }
}
