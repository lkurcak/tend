use super::{Job, filter};
use crate::colors::Tend;
use anyhow::Result;
use std::path::PathBuf;
use tabled::{
    builder::Builder,
    settings::{
        Color, Modify, Style,
        object::{Columns, Rows},
    },
};

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
                    eprintln!("Job file for {}: {}", name.job(), jobs.join(name).display());
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

    pub fn list(job_filter: &filter::Filter, verbose: bool, no_color: bool) -> Result<()> {
        let jobs = Self::jobs_dir()?;
        let mut builder = Builder::default();
        builder.push_record([
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

                let enabled = if job.enabled { "*" } else { " " };

                builder.push_record([
                    enabled,
                    &job.name,
                    &job.program,
                    &job.args.join(" "),
                    &job.working_directory.display().to_string(),
                    job.restart_behaviour(),
                    &job.group,
                ]);
            }
        }

        let mut table = builder.build();
        table.with(Style::blank());

        if table.count_rows() <= 1 {
            println!("No matching jobs found");
        } else {
            if !no_color {
                table
                    .with(
                        Modify::new(Columns::new(1..=1)).with(Color::new("\x1b[1;36m", "\x1b[0m")),
                    )
                    .with(
                        Modify::new(Columns::new(2..=2)).with(Color::new("\x1b[1;33m", "\x1b[0m")),
                    )
                    .with(Modify::new(Rows::first()).with(Color::new("\x1b[1;34m", "\x1b[0m")));
            }

            println!("{table}");
        }

        Ok(())
    }
}
