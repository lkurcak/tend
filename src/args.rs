use clap::{Parser, Subcommand};

use crate::job::event::{RestartBehavior, RestartStrategy};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Save, run, and automatically restart command-line jobs"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(long, short, help = "Print more detailed output")]
    pub verbose: bool,

    #[arg(long, help = "Disable colored output")]
    pub no_color: bool,

    #[arg(
        long,
        help = "Log retention period in days (default: 7)",
        value_name = "DAYS"
    )]
    pub log_retention: Option<u64>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(alias = "l", alias = "ls", about = "List saved jobs")]
    List {
        #[arg(help = "Job name to list", exclusive = true)]
        name: Option<String>,
        #[arg(
            short,
            long,
            help = "List all jobs",
            conflicts_with = "group",
            conflicts_with = "job"
        )]
        all: bool,
        #[arg(
            short,
            long,
            help = "List jobs in the given group(s)",
            num_args = 1..,
            conflicts_with = "all",
            use_value_delimiter = true
        )]
        group: Vec<String>,
        #[arg(
            short,
            long,
            help = "List the given job(s)",
            num_args = 1..,
            conflicts_with = "all",
            use_value_delimiter = true,
        )]
        job: Vec<String>,
        #[arg(alias = "except", short, long, help = "Omit the given job(s)", num_args = 1.., use_value_delimiter = true)]
        exclude: Vec<String>,
    },
    #[command(alias = "r", alias = "start", about = "Run saved jobs")]
    Run {
        #[arg(help = "Job name to run", exclusive = true)]
        name: Option<String>,
        #[arg(
            short,
            long,
            help = "Run all enabled jobs",
            conflicts_with = "group",
            conflicts_with = "job"
        )]
        all: bool,
        #[arg(
            short,
            long,
            help = "Run enabled jobs in the given group(s)",
            num_args = 1..,
            conflicts_with = "all",
            use_value_delimiter = true
        )]
        group: Vec<String>,
        #[arg(
            short,
            long,
            help = "Run the given job(s)",
            num_args = 1..,
            conflicts_with = "all",
            use_value_delimiter = true
        )]
        job: Vec<String>,
        #[arg(alias = "except", short, long, help = "Omit the given job(s)", num_args = 1.., use_value_delimiter = true)]
        exclude: Vec<String>,
    },
    #[command(alias = "c", alias = "new", about = "Create a job")]
    Create {
        #[arg(help = "Unique name for the job")]
        name: String,
        #[arg(help = "Executable to run")]
        program: String,
        #[arg(
            long,
            default_value = "always",
            short = 'r',
            help = "When to restart the job after it exits"
        )]
        restart: RestartBehavior,
        #[arg(
            long,
            default_value = "exponential-backoff",
            help = "How long to wait between automatic restarts"
        )]
        restart_strategy: RestartStrategy,
        #[arg(long, short = 'w', help = "Overwrite existing job with the same name")]
        overwrite: bool,
        #[arg(
            long,
            short = 'g',
            help = "Assign the job to a group",
            default_value = "default"
        )]
        group: String,
        #[arg(long, short = 't', help = "Apply a predefined job template")]
        template: Option<crate::job::template::Template>,
        #[arg(
            help = "Arguments passed to the program; use -- before program args that begin with a dash"
        )]
        args: Vec<String>,
    },
    #[command(about = "Enable jobs so they can run")]
    Enable {
        #[arg(help = "Job name to enable", exclusive = true)]
        name: Option<String>,
        #[arg(
            short,
            long,
            help = "Enable all jobs",
            conflicts_with = "group",
            conflicts_with = "job"
        )]
        all: bool,
        #[arg(
            short,
            long,
            help = "Enable jobs in the given group(s)",
            num_args = 1..,
            conflicts_with = "all",
            use_value_delimiter = true
        )]
        group: Vec<String>,
        #[arg(
            short,
            long,
            help = "Enable the given job(s)",
            num_args = 1..,
            conflicts_with = "all",
            use_value_delimiter = true
        )]
        job: Vec<String>,
        #[arg(alias = "except", short, long, help = "Omit the given job(s)", num_args = 1.., use_value_delimiter = true)]
        exclude: Vec<String>,
    },
    #[command(about = "Disable jobs so they are skipped when running jobs")]
    Disable {
        #[arg(help = "Job name to disable", exclusive = true)]
        name: Option<String>,
        #[arg(
            short,
            long,
            help = "Disable all jobs",
            conflicts_with = "group",
            conflicts_with = "job"
        )]
        all: bool,
        #[arg(
            short,
            long,
            help = "Disable jobs in the given group(s)",
            num_args = 1..,
            conflicts_with = "all",
            use_value_delimiter = true
        )]
        group: Vec<String>,
        #[arg(
            short,
            long,
            help = "Disable the given job(s)",
            num_args = 1..,
            conflicts_with = "all",
            use_value_delimiter = true
        )]
        job: Vec<String>,
        #[arg(alias = "except", short, long, help = "Omit the given job(s)", num_args = 1.., use_value_delimiter = true)]
        exclude: Vec<String>,
    },
    #[command(alias = "e", alias = "ed", about = "Edit a job")]
    Edit {
        #[arg(help = "Job name to edit")]
        name: String,
        #[command(subcommand)]
        command: EditJobCommands,
    },
    #[command(alias = "d", alias = "rm", about = "Delete jobs")]
    #[clap(group(clap::ArgGroup::new("input").required(true).args(&["name", "group", "job", "all"])))]
    Delete {
        #[arg(help = "Job name to delete", exclusive = true)]
        name: Option<String>,
        #[arg(
            short,
            long,
            help = "Delete all jobs",
            conflicts_with = "group",
            conflicts_with = "job"
        )]
        all: bool,
        #[arg(
            short,
            long,
            help = "Delete jobs in the given group(s)",
            num_args = 1..,
            conflicts_with = "all",
            use_value_delimiter = true,
        )]
        group: Vec<String>,
        #[arg(
            short,
            long,
            help = "Delete the given job(s)",
            num_args = 1..,
            conflicts_with = "all",
            use_value_delimiter = true
        )]
        job: Vec<String>,
        #[arg(alias = "except", short, long, help = "Omit the given job(s)", num_args = 1.., use_value_delimiter = true)]
        exclude: Vec<String>,
        #[arg(short, long, help = "Required with --all to confirm deletion")]
        confirm: bool,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum EditJobCommands {
    #[command(about = "Change the group of a job")]
    Group {
        #[arg(help = "New group name")]
        group: String,
    },
    #[command(about = "Manage hooks for a job")]
    Hook {
        #[command(subcommand)]
        command: EditJobHookCommands,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum EditJobHookCommands {
    #[command(
        about = "List hooks for the job",
        override_usage = "tend edit <NAME> hook list"
    )]
    List,
    #[command(
        about = "Create a hook for the job",
        override_usage = "tend edit <NAME> hook create <HOOK> <COMMAND>"
    )]
    Create {
        #[arg(help = "Hook name to create")]
        hook: String,
        #[command(subcommand)]
        t: JobHook,
    },
    #[command(
        about = "Delete a hook from the job",
        override_usage = "tend edit <NAME> hook delete <HOOK>"
    )]
    Delete {
        #[arg(help = "Hook name to delete")]
        hook: String,
    },
}

#[derive(Clone, Debug, Subcommand)]
pub enum JobHook {
    #[command(
        about = "Run an action when output contains text",
        override_usage = "tend edit <NAME> hook create <HOOK> detect-substring [OPTIONS] <SUBSTRING> <ACTION>"
    )]
    DetectSubstring {
        #[arg(help = "Text to search for")]
        substring: String,
        #[arg(help = "Action to take when the text is found")]
        action: crate::job::event::Action,
        #[arg(long, short, help = "Output stream to search", default_value = "any")]
        stream: crate::job::event::Stream,
    },
}
