use std::{
    path::{Path, PathBuf},
    // process::Child,
};
use tokio::{
    process::{Child, Command},
    sync::mpsc::Receiver,
};

use crate::colors::TendColors;
use anyhow::Result;
use prettytable::{format, row, Table};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatus {
    Running,
    Stopped,
    Failure,
    Success,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub name: String,
    pub group: String,
    pub program: String,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
    pub restart_requested: bool,
    pub restart_on_failure: bool,
    pub restart_on_success: bool,
    pub status: JobStatus,
}

impl Job {
    pub fn save(&self) -> Result<()> {
        let home = dirs_next::home_dir().ok_or(anyhow::anyhow!("Could not find home directory"))?;
        let commands = home.join(".tend").join("commands");
        std::fs::create_dir_all(&commands)?;

        let file = std::fs::File::create(commands.join(&self.name))?;
        serde_json::to_writer(file, self)?;

        Ok(())
    }

    pub fn load(name: &str) -> Result<Self> {
        let home = dirs_next::home_dir().ok_or(anyhow::anyhow!("Could not find home directory"))?;
        let commands = home.join(".tend").join("commands");

        let file = std::fs::File::open(commands.join(name))?;
        let job: Job = serde_json::from_reader(file)?;

        Ok(job)
    }

    pub fn delete(&self) -> Result<()> {
        let home = dirs_next::home_dir().ok_or(anyhow::anyhow!("Could not find home directory"))?;
        let commands = home.join(".tend").join("commands");

        std::fs::remove_file(commands.join(&self.name))?;

        Ok(())
    }

    pub fn iterate_jobs<F>(mut f: F) -> Result<()>
    where
        F: FnMut(Job),
    {
        let home = dirs_next::home_dir().ok_or(anyhow::anyhow!("Could not find home directory"))?;
        let commands = home.join(".tend").join("commands");

        for entry in std::fs::read_dir(commands)? {
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
        let home = dirs_next::home_dir().ok_or(anyhow::anyhow!("Could not find home directory"))?;
        let commands = home.join(".tend").join("commands");

        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        //table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

        table.set_titles(row![
            "Job",
            "Program",
            "Args",
            "Working Directory",
            "Restart on Failure",
            "Restart on Success",
            "Group"
        ]);

        for entry in std::fs::read_dir(commands)? {
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
                    job.restart_on_failure,
                    job.restart_on_success,
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
    pub fn create_oneshot_process(&self) -> Result<Child> {
        let mut command = self.create_command();
        let child = command.spawn()?;
        Ok(child)
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
                                if self.restart_on_success {
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
                                if self.restart_on_failure {
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
                    println!("killing process of {}", self.name.job());
                    process.kill().await?;
                    break;
                }
            }
        }

        Ok(())
    }

    #[cfg(disabled)]
    pub async fn run_once(&self) -> Result<()> {
        let mut process = self.create_oneshot_process()?;
        if let Ok(status) = process.wait().await {
            if status.success() {
                println!(
                    "{} process finished indicating {}",
                    self.name.job(),
                    "success".success()
                );
            } else {
                println!(
                    "{} process finished indicating {}",
                    self.name.job(),
                    "failure".failure()
                );
            }
        }
        Ok(())
    }
}
