use clap::{Parser, Subcommand};

use crate::job::JobRestartBehavior;

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
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(alias = "l", about = "List jobs")]
    List {
        #[arg(long, short = 'g', help = "Filter by group")]
        group: Option<String>,
    },
    #[command(alias = "r", about = "Launch jobs. Use --group or --job to filter.")]
    Run {
        #[arg(short, long, help = "Start all jobs", exclusive = true)]
        all: bool,
        #[arg(
            short,
            long,
            exclusive = true,
            help = "Start jobs from a specific group"
        )]
        group: Option<String>,
        #[arg(short, long, exclusive = true, help = "Start a specific job")]
        job: Option<String>,
    },
    // Start(StartArgs),
    #[command(alias = "c", about = "Create a job")]
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
    #[command(alias = "d", about = "Delete a job")]
    Delete { name: String },
}
