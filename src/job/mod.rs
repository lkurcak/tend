pub mod event;
pub mod filter;
pub mod io;
pub mod run;
pub mod template;

use crate::{colors::Tend, job::event::ControlFlow};
use anyhow::Result;
use folktime::Folktime;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{ChildStderr, ChildStdout, Command},
    sync::mpsc::Receiver,
};

use self::event::{Hook, RestartBehavior, RestartStrategy};

#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub name: String,
    pub group: String,
    pub program: String,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
    #[serde(default)]
    pub restart: RestartBehavior,
    #[serde(default)]
    pub restart_strategy: RestartStrategy,
    #[serde(default)]
    pub event_hooks: HashMap<String, Hook>,
    #[serde(default)]
    pub template: Option<template::Template>,
}

impl Job {
    pub const fn restart_on_success(&self) -> bool {
        match self.restart {
            RestartBehavior::Always | RestartBehavior::OnSuccess => true,
            RestartBehavior::OnFailure | RestartBehavior::Never => false,
        }
    }

    pub const fn restart_on_failure(&self) -> bool {
        match self.restart {
            RestartBehavior::Always | RestartBehavior::OnFailure => true,
            RestartBehavior::OnSuccess | RestartBehavior::Never => false,
        }
    }

    pub const fn restart_behaviour(&self) -> &'static str {
        match self.restart {
            RestartBehavior::Always => "always",
            RestartBehavior::OnSuccess => "on success",
            RestartBehavior::OnFailure => "on failure",
            RestartBehavior::Never => "never",
        }
    }
}
