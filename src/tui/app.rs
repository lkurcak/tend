use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use ratatui::widgets::TableState;
use tokio::sync::mpsc;

use crate::job::Job;

pub const MAX_OUTPUT_LINES: usize = 150;
pub const MAX_EVENTS: usize = 50;
pub const MAX_LINE_LENGTH: usize = 4096;

// === Event Types (job -> TUI communication) ===

#[derive(Debug, Clone)]
pub enum JobEvent {
    Started,
    StdoutLine(String),
    StderrLine(String),
    ProcessSuccess {
        duration: std::time::Duration,
    },
    ProcessFailure,
    Restarting {
        reason: String,
        attempt: u64,
        delay_seconds: u64,
    },
    HookTriggered {
        hook_name: String,
    },
    Stopped {
        reason: String,
    },
    SpawnError(String),
    TaskError(String),
    Finished,
}

#[derive(Debug, Clone)]
pub struct JobMessage {
    pub job_name: String,
    pub run_id: u64,
    pub event: JobEvent,
}

// === App State ===

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Jobs,
    Groups,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Running,
    Stopped,
    Restarting,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "Running"),
            Self::Stopped => write!(f, "Stopped"),
            Self::Restarting => write!(f, "Restarting"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutputLine {
    pub content: Box<str>,
    pub stream: OutputStream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
    System,
}

#[derive(Debug, Clone)]
pub struct EventEntry {
    pub message: Box<str>,
}

pub struct JobInfo {
    pub job: Job,
    pub status: JobStatus,
    pub started_at: Option<Instant>,
    pub output: VecDeque<OutputLine>,
    pub events: VecDeque<EventEntry>,
    pub log_dir: Option<PathBuf>,
    pub cancel_tx: Option<mpsc::Sender<()>>,
    pub active_run_id: Option<u64>,
}

impl JobInfo {
    pub fn uptime_str(&self) -> String {
        let Some(started) = self.started_at else {
            return String::new();
        };
        folktime::Folktime::duration(started.elapsed())
            .with_style(folktime::duration::Style::Whole)
            .with_min_unit(folktime::duration::Unit::Second)
            .with_greek_mu()
            .to_string()
    }

    fn push_output(&mut self, content: String, stream: OutputStream) {
        let truncated: Box<str> = if content.len() > MAX_LINE_LENGTH {
            let mut s = content;
            s.truncate(MAX_LINE_LENGTH);
            s.push_str("...");
            s.into_boxed_str()
        } else {
            content.into_boxed_str()
        };
        self.output.push_back(OutputLine {
            content: truncated,
            stream,
        });
        if self.output.len() > MAX_OUTPUT_LINES {
            self.output.pop_front();
        }
    }

    fn push_event(&mut self, message: String) {
        self.events.push_back(EventEntry {
            message: message.into_boxed_str(),
        });
        if self.events.len() > MAX_EVENTS {
            self.events.pop_front();
        }
    }

    fn clear_active_run(&mut self) {
        self.status = JobStatus::Stopped;
        self.started_at = None;
        self.cancel_tx = None;
        self.active_run_id = None;
    }
}

pub struct GroupInfo {
    pub name: String,
    pub expanded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupListItem {
    Group(usize),
    Job(usize),
}

pub enum AppCommand {
    StartJob(usize),
    StopJob(usize),
    RestartJob(usize),
    OpenLogs(usize),
    Quit,
}

pub struct App {
    pub tab: Tab,
    pub should_quit: bool,
    pub jobs: Vec<JobInfo>,
    pub job_list_state: TableState,
    pub groups: Vec<GroupInfo>,
    pub group_list_state: TableState,
    pub group_list_items: Vec<GroupListItem>,
    pub event_rx: mpsc::Receiver<JobMessage>,
    pub output_scroll_offset: u16,
    pub next_run_id: u64,
}

impl App {
    pub fn new(jobs: Vec<Job>, event_rx: mpsc::Receiver<JobMessage>) -> Self {
        let mut job_infos = Vec::new();
        let mut group_names: Vec<String> = Vec::new();

        for job in jobs {
            if !group_names.contains(&job.group) {
                group_names.push(job.group.clone());
            }
            job_infos.push(JobInfo {
                job,
                status: JobStatus::Stopped,
                started_at: None,
                output: VecDeque::new(),
                events: VecDeque::new(),
                log_dir: None,
                cancel_tx: None,
                active_run_id: None,
            });
        }

        let groups: Vec<GroupInfo> = group_names
            .into_iter()
            .map(|name| GroupInfo {
                name,
                expanded: false,
            })
            .collect();

        let mut app = Self {
            tab: Tab::Jobs,
            should_quit: false,
            jobs: job_infos,
            job_list_state: TableState::default(),
            groups,
            group_list_state: TableState::default(),
            group_list_items: Vec::new(),
            event_rx,
            output_scroll_offset: 0,
            next_run_id: 1,
        };

        if !app.jobs.is_empty() {
            app.job_list_state.select(Some(0));
        }

        app.rebuild_group_list();
        if !app.group_list_items.is_empty() {
            app.group_list_state.select(Some(0));
        }

        app
    }

    pub const fn next_run_id(&mut self) -> u64 {
        let run_id = self.next_run_id;
        self.next_run_id = self.next_run_id.saturating_add(1);
        run_id
    }

    pub fn rebuild_group_list(&mut self) {
        self.group_list_items.clear();
        for (gi, group) in self.groups.iter().enumerate() {
            self.group_list_items.push(GroupListItem::Group(gi));
            if group.expanded {
                for (ji, job_info) in self.jobs.iter().enumerate() {
                    if job_info.job.group == group.name {
                        self.group_list_items.push(GroupListItem::Job(ji));
                    }
                }
            }
        }
    }

    pub fn selected_job_index(&self) -> Option<usize> {
        match self.tab {
            Tab::Jobs => self.job_list_state.selected(),
            Tab::Groups => {
                let selected = self.group_list_state.selected()?;
                match self.group_list_items.get(selected)? {
                    GroupListItem::Job(idx) => Some(*idx),
                    GroupListItem::Group(_) => None,
                }
            }
        }
    }

    pub fn selected_group_index(&self) -> Option<usize> {
        if self.tab != Tab::Groups {
            return None;
        }
        let selected = self.group_list_state.selected()?;
        match self.group_list_items.get(selected)? {
            GroupListItem::Group(idx) => Some(*idx),
            GroupListItem::Job(_) => None,
        }
    }

    pub fn group_job_counts(&self, group_name: &str) -> (usize, usize) {
        let mut running = 0;
        let mut total = 0;
        for job_info in &self.jobs {
            if job_info.job.group == group_name {
                total += 1;
                if job_info.status == JobStatus::Running || job_info.status == JobStatus::Restarting
                {
                    running += 1;
                }
            }
        }
        (running, total)
    }

    pub fn jobs_in_group(&self, group_index: usize) -> Vec<usize> {
        let Some(group_name) = self.groups.get(group_index).map(|group| &group.name) else {
            return Vec::new();
        };
        self.jobs
            .iter()
            .enumerate()
            .filter(|(_, ji)| ji.job.group == *group_name)
            .map(|(i, _)| i)
            .collect()
    }

    pub fn handle_job_event(&mut self, msg: JobMessage) {
        let Some(job_index) = self.jobs.iter().position(|j| j.job.name == msg.job_name) else {
            return;
        };

        let is_selected = self
            .selected_job_index()
            .and_then(|idx| self.jobs.get(idx))
            .is_some_and(|job_info| job_info.job.name == msg.job_name);

        let Some(job_info) = self.jobs.get_mut(job_index) else {
            return;
        };

        if job_info.active_run_id != Some(msg.run_id) {
            return;
        }

        match msg.event {
            JobEvent::Started => {
                job_info.status = JobStatus::Running;
                job_info.started_at = Some(Instant::now());
                if job_info.log_dir.is_none() {
                    job_info.log_dir = crate::job::log::job_log_dir(&job_info.job.name).ok();
                }
                job_info.push_event("Started".to_string());
                job_info.push_output("--- Process started ---".to_string(), OutputStream::System);
            }
            JobEvent::StdoutLine(line) => {
                job_info.push_output(line, OutputStream::Stdout);
            }
            JobEvent::StderrLine(line) => {
                job_info.push_output(line, OutputStream::Stderr);
            }
            JobEvent::ProcessSuccess { duration } => {
                let dur_str = folktime::Folktime::duration(duration)
                    .with_min_unit(folktime::duration::Unit::Second)
                    .with_greek_mu()
                    .to_string();
                job_info.push_event(format!("Exited successfully after {dur_str}"));
            }
            JobEvent::ProcessFailure => {
                job_info.push_event("Exited with failure".to_string());
            }
            JobEvent::Restarting {
                reason,
                attempt,
                delay_seconds,
            } => {
                job_info.status = JobStatus::Restarting;
                let message = if delay_seconds > 0 {
                    format!("Restarting in {delay_seconds}s ({reason}, #{attempt})")
                } else {
                    format!("Restarting ({reason}, #{attempt})")
                };
                job_info.push_event(message.clone());
                job_info.push_output(format!("--- {message} ---"), OutputStream::System);
            }
            JobEvent::HookTriggered { hook_name } => {
                job_info.push_event(format!("Hook triggered: {hook_name}"));
            }
            JobEvent::Stopped { reason } => {
                job_info.clear_active_run();
                job_info.push_event(format!("Stopped ({reason})"));
                job_info.push_output(format!("--- Stopped ({reason}) ---"), OutputStream::System);
            }
            JobEvent::SpawnError(error) => {
                job_info.clear_active_run();
                job_info.push_event(format!("Failed to start: {error}"));
                job_info.push_output(
                    format!("--- Failed to start: {error} ---"),
                    OutputStream::System,
                );
            }
            JobEvent::TaskError(error) => {
                job_info.clear_active_run();
                job_info.push_event(format!("Task failed: {error}"));
                job_info.push_output(
                    format!("--- Task failed: {error} ---"),
                    OutputStream::System,
                );
            }
            JobEvent::Finished => {
                job_info.clear_active_run();
            }
        }

        if is_selected {
            self.output_scroll_offset = 0;
        }
    }

    pub fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Vec<AppCommand> {
        use crossterm::event::{KeyCode, KeyModifiers};

        let mut commands = Vec::new();

        // Global keybindings
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                commands.push(AppCommand::Quit);
                return commands;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                commands.push(AppCommand::Quit);
                return commands;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.tab = match self.tab {
                    Tab::Jobs => Tab::Groups,
                    Tab::Groups => Tab::Jobs,
                };
                self.output_scroll_offset = 0;
                return commands;
            }
            KeyCode::Char('1') => {
                self.tab = Tab::Jobs;
                self.output_scroll_offset = 0;
                return commands;
            }
            KeyCode::Char('2') => {
                self.tab = Tab::Groups;
                self.output_scroll_offset = 0;
                return commands;
            }
            _ => {}
        }

        match self.tab {
            Tab::Jobs => self.handle_jobs_key(key, &mut commands),
            Tab::Groups => self.handle_groups_key(key, &mut commands),
        }

        commands
    }

    fn handle_jobs_key(&mut self, key: crossterm::event::KeyEvent, commands: &mut Vec<AppCommand>) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_job_selection(-1);
                self.output_scroll_offset = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_job_selection(1);
                self.output_scroll_offset = 0;
            }
            KeyCode::Char('g') => {
                if !self.jobs.is_empty() {
                    self.job_list_state.select(Some(0));
                    self.output_scroll_offset = 0;
                }
            }
            KeyCode::Char('G') => {
                if !self.jobs.is_empty() {
                    self.job_list_state.select(Some(self.jobs.len() - 1));
                    self.output_scroll_offset = 0;
                }
            }
            KeyCode::Char('s') | KeyCode::Enter => {
                if let Some(idx) = self.job_list_state.selected() {
                    if idx < self.jobs.len() && self.jobs[idx].status == JobStatus::Stopped {
                        commands.push(AppCommand::StartJob(idx));
                    }
                }
            }
            KeyCode::Char('x') => {
                if let Some(idx) = self.job_list_state.selected() {
                    if idx < self.jobs.len() && self.jobs[idx].status != JobStatus::Stopped {
                        commands.push(AppCommand::StopJob(idx));
                    }
                }
            }
            KeyCode::Char('r') => {
                if let Some(idx) = self.job_list_state.selected() {
                    if idx < self.jobs.len() {
                        commands.push(AppCommand::RestartJob(idx));
                    }
                }
            }
            KeyCode::Char('l') => {
                if let Some(idx) = self.job_list_state.selected() {
                    if idx < self.jobs.len() {
                        commands.push(AppCommand::OpenLogs(idx));
                    }
                }
            }
            KeyCode::PageUp => {
                self.output_scroll_offset = self.output_scroll_offset.saturating_add(10);
            }
            KeyCode::PageDown => {
                self.output_scroll_offset = self.output_scroll_offset.saturating_sub(10);
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_lines)]
    fn handle_groups_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        commands: &mut Vec<AppCommand>,
    ) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_group_selection(-1);
                self.output_scroll_offset = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_group_selection(1);
                self.output_scroll_offset = 0;
            }
            KeyCode::Char('g') => {
                if !self.group_list_items.is_empty() {
                    self.group_list_state.select(Some(0));
                    self.output_scroll_offset = 0;
                }
            }
            KeyCode::Char('G') => {
                if !self.group_list_items.is_empty() {
                    self.group_list_state
                        .select(Some(self.group_list_items.len() - 1));
                    self.output_scroll_offset = 0;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(selected) = self.group_list_state.selected() {
                    if let Some(item) = self.group_list_items.get(selected).copied() {
                        match item {
                            GroupListItem::Group(gi) => {
                                self.groups[gi].expanded = !self.groups[gi].expanded;
                                self.rebuild_group_list();
                                // Keep selection on the same group
                                let new_pos = self.group_list_items.iter().position(
                                    |i| matches!(i, GroupListItem::Group(idx) if *idx == gi),
                                );
                                if let Some(pos) = new_pos {
                                    self.group_list_state.select(Some(pos));
                                }
                            }
                            GroupListItem::Job(ji) => {
                                if self.jobs[ji].status == JobStatus::Stopped {
                                    commands.push(AppCommand::StartJob(ji));
                                } else {
                                    commands.push(AppCommand::StopJob(ji));
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('s') => {
                if let Some(selected) = self.group_list_state.selected() {
                    if let Some(item) = self.group_list_items.get(selected).copied() {
                        match item {
                            GroupListItem::Group(gi) => {
                                let job_indices = self.jobs_in_group(gi);
                                for idx in job_indices {
                                    if self.jobs[idx].status == JobStatus::Stopped {
                                        commands.push(AppCommand::StartJob(idx));
                                    }
                                }
                            }
                            GroupListItem::Job(ji) => {
                                if self.jobs[ji].status == JobStatus::Stopped {
                                    commands.push(AppCommand::StartJob(ji));
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('x') => {
                if let Some(selected) = self.group_list_state.selected() {
                    if let Some(item) = self.group_list_items.get(selected).copied() {
                        match item {
                            GroupListItem::Group(gi) => {
                                let job_indices = self.jobs_in_group(gi);
                                for idx in job_indices {
                                    if self.jobs[idx].status != JobStatus::Stopped {
                                        commands.push(AppCommand::StopJob(idx));
                                    }
                                }
                            }
                            GroupListItem::Job(ji) => {
                                if self.jobs[ji].status != JobStatus::Stopped {
                                    commands.push(AppCommand::StopJob(ji));
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('r') => {
                if let Some(selected) = self.group_list_state.selected() {
                    if let Some(item) = self.group_list_items.get(selected).copied() {
                        match item {
                            GroupListItem::Group(gi) => {
                                let job_indices = self.jobs_in_group(gi);
                                for idx in job_indices {
                                    commands.push(AppCommand::RestartJob(idx));
                                }
                            }
                            GroupListItem::Job(ji) => {
                                commands.push(AppCommand::RestartJob(ji));
                            }
                        }
                    }
                }
            }
            KeyCode::Char('l') => {
                if let Some(selected) = self.group_list_state.selected() {
                    if let Some(GroupListItem::Job(ji)) =
                        self.group_list_items.get(selected).copied()
                    {
                        commands.push(AppCommand::OpenLogs(ji));
                    }
                }
            }
            KeyCode::PageUp => {
                self.output_scroll_offset = self.output_scroll_offset.saturating_add(10);
            }
            KeyCode::PageDown => {
                self.output_scroll_offset = self.output_scroll_offset.saturating_sub(10);
            }
            _ => {}
        }
    }

    fn move_job_selection(&mut self, delta: i32) {
        let len = self.jobs.len();
        if len == 0 {
            return;
        }
        let current = self.job_list_state.selected().unwrap_or(0);
        #[allow(clippy::cast_sign_loss)]
        let new = if delta > 0 {
            (current + delta as usize).min(len - 1)
        } else {
            current.saturating_sub((-delta) as usize)
        };
        self.job_list_state.select(Some(new));
    }

    fn move_group_selection(&mut self, delta: i32) {
        let len = self.group_list_items.len();
        if len == 0 {
            return;
        }
        let current = self.group_list_state.selected().unwrap_or(0);
        #[allow(clippy::cast_sign_loss)]
        let new = if delta > 0 {
            (current + delta as usize).min(len - 1)
        } else {
            current.saturating_sub((-delta) as usize)
        };
        self.group_list_state.select(Some(new));
    }
}
