use clap::{Parser, Subcommand};

use crate::job::{JobRestartBehavior, JobRestartStrategy};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Quickly spin up/down groups of command-line tasks with automated recovery"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, short, help = "Print extra information")]
    pub verbose: bool,

    #[arg(long, help = "Disable color output")]
    pub no_color: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(alias = "l", alias = "ls", about = "List jobs")]
    List {
        #[arg(short, long, help = "List all jobs", exclusive = true)]
        all: bool,
        #[arg(
            short,
            long,
            help = "List jobs from specific group(s)",
            num_args = 1..,
            use_value_delimiter = true
        )]
        group: Vec<String>,
        #[arg(short, long, help = "List specific job(s)", num_args = 1.., use_value_delimiter = true)]
        job: Vec<String>,
        #[arg(alias = "except", short, long, help = "Exclude specific job(s)", num_args = 1.., use_value_delimiter = true)]
        exclude: Vec<String>,
    },
    #[command(alias = "r", alias = "start", about = "Start jobs")]
    Run {
        #[arg(short, long, help = "Start all jobs", exclusive = true)]
        all: bool,
        #[arg(
            short,
            long,
            help = "Start jobs from specific group(s)",
            num_args = 1..,
            use_value_delimiter = true
        )]
        group: Vec<String>,
        #[arg(short, long, help = "Start specific job(s)", num_args = 1.., use_value_delimiter = true)]
        job: Vec<String>,
        #[arg(alias = "except", short, long, help = "Exclude specific job(s)", num_args = 1.., use_value_delimiter = true)]
        exclude: Vec<String>,
    },
    #[command(alias = "c", alias = "new", about = "Create a job")]
    Create {
        #[arg(help = "Name of the job. Must be unique.")]
        name: String,
        #[arg(help = "Program to run. Must be in PATH or otherwise accessible.")]
        program: String,
        #[arg(
            long,
            default_value = "on-failure",
            short = 'r',
            help = "Restart condition"
        )]
        restart: JobRestartBehavior,
        #[arg(long, default_value = "exponential-backoff", help = "Restart strategy")]
        restart_strategy: JobRestartStrategy,
        #[arg(long, short = 'w', help = "Overwrite existing job with the same name")]
        overwrite: bool,
        #[arg(
            long,
            short = 'g',
            help = "Add job to a group",
            default_value = "default"
        )]
        group: String,
        #[arg(help = "Use -- to separate program arguments from job arguments.")]
        args: Vec<String>,
    },
    #[command(alias = "d", alias = "rm", about = "Delete jobs")]
    #[clap(group(clap::ArgGroup::new("input").required(true).args(&["name", "group", "job", "all"])))]
    Delete {
        #[arg(help = "Name of the job to delete", exclusive = true)]
        name: Option<String>,
        #[arg(
            short,
            long,
            help = "Delete all jobs",
            conflicts_with = "name",
            conflicts_with = "group",
            conflicts_with = "job"
        )]
        all: bool,
        #[arg(
            short,
            long,
            help = "Delete jobs from specific group(s)",
            num_args = 1..,
            use_value_delimiter = true
        )]
        group: Vec<String>,
        #[arg(short, long, help = "Delete specific job(s)", num_args = 1.., use_value_delimiter = true)]
        job: Vec<String>,
        #[arg(alias = "except", short, long, help = "Exclude specific job(s)", num_args = 1.., use_value_delimiter = true)]
        exclude: Vec<String>,
        #[arg(short, long, help = "Confirm delete action")]
        confirm: bool,
    },
}
