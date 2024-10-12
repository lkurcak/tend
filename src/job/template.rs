use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use super::{event::Action, event::Event, event::Stream, Hook, Job};

#[derive(Copy, Clone, Debug, ValueEnum, Serialize, Deserialize)]
pub enum Template {
    PortForward,
}

impl Job {
    pub fn apply_template(&mut self, template: Template) {
        match template {
            Template::PortForward => {
                self.event_hooks.push(Hook {
                    name: "error hook".to_string(),
                    event: Event::DetectSubstring {
                        contains: "error".to_string(),
                        stream: Stream::Any,
                    },
                    action: Action::Restart,
                });
                self.event_hooks.push(Hook {
                    name: "aborted hook".to_string(),
                    event: Event::DetectSubstring {
                        contains: "aborted".to_string(),
                        stream: Stream::Any,
                    },
                    action: Action::Restart,
                });
                self.event_hooks.push(Hook {
                    name: "connection lost hook".to_string(),
                    event: Event::DetectSubstring {
                        contains: "connection lost".to_string(),
                        stream: Stream::Any,
                    },
                    action: Action::Restart,
                });
            }
        }
    }
}
