use super::{
    AsyncBufReadExt, BufReader, ChildStderr, ChildStdout, Command, ControlFlow, Folktime, Job,
    Lines, Receiver, Result, Tend,
};

impl Job {
    fn create_command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.current_dir(&self.working_directory);
        command.args(&self.args);
        command
    }

    async fn wait_for_something(
        &self,
        process: &mut tokio::process::Child,
        rx: &mut Receiver<()>,
        verbose: bool,
        stdout: &mut Lines<BufReader<ChildStdout>>,
        stderr: &mut Lines<BufReader<ChildStderr>>,
    ) -> Result<ControlFlow> {
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
        let mut command = self.create_command();

        // let mut successes = 0;
        // let mut failures = 0;
        let mut backoff_restart_count = 0;

        'job: loop {
            if verbose {
                println!("{} starting", self.name.job(),);
            }
            let start_time = std::time::Instant::now();
            let mut process = command
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;

            let mut stdout = BufReader::new(
                process
                    .stdout
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("Could not get stdout"))?,
            )
            .lines();

            let mut stderr = BufReader::new(
                process
                    .stderr
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
                        let _ = process.kill().await;

                        let delay_seconds =
                            self.restart_strategy.delay_seconds(backoff_restart_count);
                        if delay_seconds != 0 {
                            println!(
                                " (restarting in {} seconds)",
                                delay_seconds.to_string().time_value()
                            );
                            tokio::time::sleep(tokio::time::Duration::from_secs(delay_seconds))
                                .await;
                        } else {
                            println!(" (restarting)");
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
                        let _ = process.kill().await;
                        break 'job;
                    }
                }
            }
        }

        Ok(())
    }
}
