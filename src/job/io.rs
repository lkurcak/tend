use super::{filter, Job};
use crate::colors::Tend;
use anyhow::Result;
use prettytable::{format, row, Table};
use std::path::PathBuf;

impl Job {
    fn jobs_dir() -> Result<PathBuf> {
        let home = dirs_next::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
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

    pub fn load(name: &str) -> Option<Self> {
        let jobs = Self::jobs_dir().ok()?;
        let file = std::fs::File::open(jobs.join(name)).ok()?;

        let job: Result<Self, _> = serde_json::from_reader(file);
        match job {
            Ok(mut job) => {
                if let Some(template) = job.template {
                    job.apply_template(template);
                }

                Some(job)
            }
            Err(e) => {
                eprintln!("Error loading {}: {}", name.job(), e.to_string().failure());
                None
            }
        }
    }

    pub fn delete(&self) -> Result<()> {
        let jobs = Self::jobs_dir()?;
        std::fs::remove_file(jobs.join(&self.name))?;

        Ok(())
    }

    pub fn iterate_jobs_filtered<F>(mut f: F, filter: &filter::Filter) -> Result<()>
    where
        F: FnMut(Self),
    {
        let jobs = Self::jobs_dir()?;
        for entry in std::fs::read_dir(jobs)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let Some(job) = Self::load(path.file_name().unwrap().to_str().unwrap()) else {
                    continue;
                };
                if filter.matches(&job) {
                    f(job);
                }
            }
        }

        Ok(())
    }

    pub fn list(job_filter: &filter::Filter) -> Result<()> {
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
                let name = path.file_name().unwrap().to_str().unwrap();
                let Some(job) = Self::load(name) else {
                    continue;
                };

                if !job_filter.matches(&job) {
                    continue;
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
