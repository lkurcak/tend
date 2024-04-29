use crate::colors::TendColors;
use anyhow::Result;
use prettytable::{format, row, Table};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::{process::Command, sync::mpsc::Receiver};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub name: String,
    pub group: String,
    pub program: String,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
    pub restart: JobRestartBehavior,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum, Copy, PartialEq, Eq)]
pub enum JobRestartBehavior {
    Always,
    OnSuccess,
    OnFailure,
    Never,
}

pub enum JobFilter {
    All,
    Job { job: String },
    Group { group: String },
}

impl Job {
    fn jobs_dir() -> Result<PathBuf> {
        let home = dirs_next::home_dir().ok_or(anyhow::anyhow!("Could not find home directory"))?;
        let jobs = home.join(".tend").join("jobs");
        std::fs::create_dir_all(&jobs)?;
        Ok(jobs)
    }

    pub fn save(&self, overwrite: bool) -> Result<()> {
        let jobs = Self::jobs_dir()?;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(overwrite)
            .create_new(!overwrite)
            .open(jobs.join(&self.name))?;
        // serde_json::to_writer(file, self)?;
        serde_json::to_writer_pretty(file, self)?;

        Ok(())
    }

    pub fn load(name: &str) -> Result<Self> {
        let jobs = Self::jobs_dir()?;
        let file = std::fs::File::open(jobs.join(name))?;
        let job: Job = serde_json::from_reader(file)?;

        Ok(job)
    }

    pub fn delete(&self) -> Result<()> {
        let jobs = Self::jobs_dir()?;
        std::fs::remove_file(jobs.join(&self.name))?;

        Ok(())
    }

    pub fn iterate_jobs<F>(mut f: F) -> Result<()>
    where
        F: FnMut(Job),
    {
        let jobs = Self::jobs_dir()?;
        for entry in std::fs::read_dir(jobs)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let job: Job = serde_json::from_reader(std::fs::File::open(&path)?)?;
                f(job);
            }
        }

        Ok(())
    }

    pub fn list(group: Option<String>) -> Result<()> {
        let jobs = Self::jobs_dir()?;
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        //table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

        table.set_titles(row![
            "Job",
            "Program",
            "Args",
            "Working Directory",
            "Restart",
            "Group"
        ]);

        for entry in std::fs::read_dir(jobs)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let job: Job = serde_json::from_reader(std::fs::File::open(&path)?)?;
                if let Some(ref group) = group {
                    if &job.group != group {
                        continue;
                    }
                }

                table.add_row(row![
                    job.name.job(),
                    job.program.program(),
                    job.args.join(" "),
                    job.working_directory.display(),
                    job.restart_behaviour(),
                    job.group,
                ]);
            }
        }

        if table.is_empty() {
            println!("No jobs found");
        } else {
            table.printstd();
        }

        Ok(())
    }
}

impl Job {
    fn create_command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.current_dir(&self.working_directory);
        command.args(&self.args);
        command
    }

    pub async fn create_repeated_process(self, mut rx: Receiver<()>) -> Result<()> {
        let mut command = self.create_command();

        loop {
            let mut process = command.spawn()?;
            tokio::select! {
                a = process.wait() => {
                    match a {
                        Ok(status) => {
                            if status.success() {
                                println!(
                                    "{} process finished indicating {}",
                                    self.name.job(),
                                    "success".success()
                                );
                                if self.restart_on_success() {
                                    println!("{} restarting", self.name.job());
                                } else {
                                    println!("{} stopping", self.name.job());
                                    break;
                                }
                            } else {
                                println!(
                                    "{} process finished indicating {}",
                                    self.name.job(),
                                    "failure".failure()
                                );
                                if self.restart_on_failure() {
                                    println!("{} restarting", self.name.job());
                                } else {
                                    println!("{} stopping", self.name.job());
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            println!(
                                "{} could not be awaited ({:?})",
                                self.name.job(),
                                e.to_string().failure()
                            );
                        }
                    }
                }
                _ = rx.recv() => {
                    // println!("killing process of {}", self.name.job());
                    process.kill().await?;
                    break;
                }
            }
        }

        Ok(())
    }
}

impl Job {
    pub fn restart_on_success(&self) -> bool {
        match self.restart {
            JobRestartBehavior::Always | JobRestartBehavior::OnSuccess => true,
            JobRestartBehavior::OnFailure | JobRestartBehavior::Never => false,
        }
    }

    pub fn restart_on_failure(&self) -> bool {
        match self.restart {
            JobRestartBehavior::Always | JobRestartBehavior::OnFailure => true,
            JobRestartBehavior::OnSuccess | JobRestartBehavior::Never => false,
        }
    }

    pub fn restart_behaviour(&self) -> &'static str {
        match self.restart {
            JobRestartBehavior::Always => "always",
            JobRestartBehavior::OnSuccess => "on success",
            JobRestartBehavior::OnFailure => "on failure",
            JobRestartBehavior::Never => "never",
        }
    }
}
