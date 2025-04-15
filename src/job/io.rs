use super::{Job, filter};
use crate::colors::Tend;
use anyhow::Result;
use prettytable::{Table, format, row};
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

    pub fn load(name: &str, verbose: bool) -> Option<Self> {
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
                eprintln!("{} {}: {}", name.job(), "could not be loaded".failure(), e);
                if verbose {
                    eprintln!(
                        "{} is located at: {}",
                        name.job(),
                        jobs.join(name).display()
                    );
                }
                None
            }
        }
    }

    pub fn delete_all_unchecked() -> Result<()> {
        let jobs = Self::jobs_dir()?;
        for entry in std::fs::read_dir(jobs)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(path)?;
            }
        }

        Ok(())
    }

    pub fn delete_unchecked(name: &str) -> Result<()> {
        let jobs = Self::jobs_dir()?;
        std::fs::remove_file(jobs.join(name))?;

        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        Self::delete_unchecked(&self.name)
    }

    pub fn iterate_job_names_filtered<F>(
        mut f: F,
        filter: &filter::Filter,
        _verbose: bool,
    ) -> Result<()>
    where
        F: FnMut(&str),
    {
        let jobs = Self::jobs_dir()?;
        for entry in std::fs::read_dir(jobs)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let Some(job_name) = path.file_name().unwrap().to_str() else {
                    continue;
                };
                if filter.matches_name(job_name) {
                    f(job_name);
                }
            }
        }

        Ok(())
    }

    pub fn iterate_jobs_filtered<F>(
        mut f: F,
        filter: &filter::Filter,
        include_disabled: bool,
        verbose: bool,
    ) -> Result<()>
    where
        F: FnMut(Self),
    {
        let jobs = Self::jobs_dir()?;
        for entry in std::fs::read_dir(jobs)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap().to_str().unwrap();
                let Some(job) = Self::load(name, verbose) else {
                    continue;
                };

                if !include_disabled && !job.enabled {
                    continue;
                }

                if !filter.matches(&job) {
                    continue;
                }

                f(job);
            }
        }

        Ok(())
    }

    pub fn list(job_filter: &filter::Filter, verbose: bool) -> Result<()> {
        let jobs = Self::jobs_dir()?;
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_CLEAN);

        table.set_titles(row![FB =>
            "ENABLED",
            "JOB",
            "PROGRAM",
            "ARGS",
            "WORKING DIRECTORY",
            "RESTART",
            "GROUP",
        ]);

        for entry in std::fs::read_dir(jobs)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap().to_str().unwrap();
                let Some(job) = Self::load(name, verbose) else {
                    continue;
                };

                if !job_filter.matches(&job) {
                    continue;
                }

                table.add_row(row![
                    r->if job.enabled { "*" } else { " " },
                    bFC->&job.name,
                    bFY->&job.program,
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
