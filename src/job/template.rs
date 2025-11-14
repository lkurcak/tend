use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use super::{
    Hook, Job,
    event::{Action, Event, RestartStrategy, Stream},
};

#[derive(Copy, Clone, Debug, ValueEnum, Serialize, Deserialize)]
pub enum Template {
    PortForward,
}

impl Job {
    pub fn apply_template(&mut self, template: Template) {
        match template {
            Template::PortForward => {
                self.restart_strategy = RestartStrategy::ExponentialBackoff;

                self.event_hooks.push(Hook {
                    name: "lost connection hook".to_string(),
                    event: Event::DetectSubstring {
                        contains: "lost connection to pod".to_string(),
                        stream: Stream::Any,
                    },
                    action: Action::FastRestart,
                });

                self.event_hooks.push(Hook {
                    name: "pending hook".to_string(),
                    event: Event::DetectSubstring {
                        contains: "Current status=Pending".to_string(),
                        stream: Stream::Any,
                    },
                    action: Action::FastRestart,
                });

                self.event_hooks.push(Hook {
                    name: "timeout hook".to_string(),
                    event: Event::DetectSubstring {
                        contains: "Timeout occurred".to_string(),
                        stream: Stream::Any,
                    },
                    action: Action::FastRestart,
                });

                self.event_hooks.push(Hook {
                    name: "container not running hook".to_string(),
                    event: Event::DetectSubstring {
                        contains: "container not running".to_string(),
                        stream: Stream::Any,
                    },
                    action: Action::Restart,
                });
            }
        }
    }
}
