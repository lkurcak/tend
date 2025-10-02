use crate::colors::Tend;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use super::Job;

#[derive(PartialEq, Eq)]
pub enum ControlFlow<'a> {
    Nothing,
    FastRestartCommand(&'a str),
    RestartCommand(&'a str),
    StopJob(&'a str),
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, clap::ValueEnum, Copy, PartialEq, Eq)]
pub enum RestartBehavior {
    #[default]
    Always,
    OnSuccess,
    OnFailure,
    Never,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, clap::ValueEnum, Copy, PartialEq, Eq)]
pub enum RestartStrategy {
    Immediate,
    #[default]
    ExponentialBackoff,
}

impl RestartStrategy {
    pub fn delay_seconds(self, restarts: u64) -> u64 {
        match self {
            Self::Immediate => 0,
            Self::ExponentialBackoff => [0, 0, 0, 1, 2, 4, 8, 15, 30]
                .get(usize::try_from(restarts).unwrap_or(0))
                .copied()
                .unwrap_or(60),
        }
    }

    pub fn delay_seconds_fast(self, restarts: u64) -> u64 {
        match self {
            Self::Immediate => 0,
            Self::ExponentialBackoff => [0, 0, 0, 1, 2, 3, 5]
                .get(usize::try_from(restarts).unwrap_or(0))
                .copied()
                .unwrap_or(8),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum, Default)]
pub enum Stream {
    Stdout,
    Stderr,
    #[default]
    Any,
}

/// TODO: Rework [`Job::restart`] to use this instead of [`JobRestartStrategy`]
#[derive(Debug, Clone, Serialize, Deserialize, clap::Parser)]
pub enum Event {
    // FinishedSuccess,
    // FinishedFailure,
    DetectSubstring { stream: Stream, contains: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
pub enum Action {
    Restart,
    FastRestart,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub name: String,
    pub event: Event,
    pub action: Action,
}

impl Job {
    pub fn stdout_line_callback<'a>(&'a self, line: &str, verbose: bool) -> ControlFlow<'a> {
        for hook in &self.event_hooks {
            let Hook {
                name,
                event: Event::DetectSubstring { stream, contains },
                action,
            } = hook;

            let detection = match stream {
                Stream::Any | Stream::Stdout => line.contains(contains),
                Stream::Stderr => false,
            };

            if detection {
                if verbose {
                    println!("{} triggered hook {:?}", self.name.job(), hook);
                }

                return match action {
                    Action::Restart => ControlFlow::RestartCommand(name),
                    Action::FastRestart => ControlFlow::FastRestartCommand(name),
                    Action::Stop => ControlFlow::StopJob(name),
                };
            }
        }

        ControlFlow::Nothing
    }

    pub fn stderr_line_callback<'a>(&'a self, line: &str, verbose: bool) -> ControlFlow<'a> {
        for hook in &self.event_hooks {
            let Hook {
                name,
                event: Event::DetectSubstring { stream, contains },
                action,
            } = hook;

            let detection = match stream {
                Stream::Stdout => false,
                Stream::Any | Stream::Stderr => line.contains(contains),
            };

            if detection {
                if verbose {
                    println!("{} triggered hook {:?}", self.name.job(), hook);
                }

                return match action {
                    Action::Restart => ControlFlow::RestartCommand(name),
                    Action::FastRestart => ControlFlow::FastRestartCommand(name),
                    Action::Stop => ControlFlow::StopJob(name),
                };
            }
        }

        ControlFlow::Nothing
    }
}
