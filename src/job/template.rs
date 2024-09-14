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
            // @note: Examples of errors from `kubectl port-forward`:
            // E0515 11:45:17.837897   23508 portforward.go:372] error copying from remote stream to local connection: readfrom tcp4 127.0.0.1:8443->127.0.0.1:50656: write tcp4 127.0.0.1:8443->127.0.0.1:50656: wsasend: An established connection was aborted by the software in your host machine.
            // E0515 11:45:51.137714   23508 portforward.go:340] error creating error stream for port 8443 -> 8443: Timeout occurred
            // E0515 11:45:51.293626   23508 portforward.go:362] error creating forwarding stream for port 8443 -> 8443: Timeout occurred
            // E0515 11:45:52.013842   23508 portforward.go:362] error creating forwarding stream for port 8443 -> 8443: Timeout occurred
            // E0515 11:46:53.524413   23508 portforward.go:400] an error occurred forwarding 8443 -> 8443: error forwarding port 8443 to pod 20919150d2fddf20d4b94e389744ffde70ae784debf216326d58c7dd0d79401e, uid : failed to execute portforward in network namespace "/var/run/netns/cni-d0e0bbba-6286-aba6-45a1-fa24e0e614e2": failed to connect to localhost:8443 inside namespace "20919150d2fddf20d4b94e389744ffde70ae784debf216326d58c7dd0d79401e", IPv4: dial tcp4 127.0.0.1:8443: connect: connection refused IPv6 dial tcp6: address localhost: no suitable address found
            // E0515 11:46:53.608922   23508 portforward.go:400] an error occurred forwarding 8443 -> 8443: error forwarding port 8443 to pod 20919150d2fddf20d4b94e389744ffde70ae784debf216326d58c7dd0d79401e, uid : failed to execute portforward in network namespace "/var/run/netns/cni-d0e0bbba-6286-aba6-45a1-fa24e0e614e2": failed to connect to localhost:8443 inside namespace "20919150d2fddf20d4b94e389744ffde70ae784debf216326d58c7dd0d79401e", IPv4: dial tcp4 127.0.0.1:8443: connect: connection refused IPv6 dial tcp6: address localhost: no suitable address found
            // E0515 11:47:03.229256   23508 portforward.go:340] error creating error stream for port 8443 -> 8443: Timeout occurred
            // E0515 11:47:07.613031   23508 portforward.go:340] error creating error stream for port 8443 -> 8443: Timeout occurred
            // E0515 11:47:23.206203   23508 portforward.go:340] error creating error stream for port 8443 -> 8443: Timeout occurred
            // E0515 11:47:23.409211   23508 portforward.go:362] error creating forwarding stream for port 8443 -> 8443: Timeout occurred
            // E0515 11:47:23.786188   23508 portforward.go:400] an error occurred forwarding 8443 -> 8443: error forwarding port 8443 to pod 20919150d2fddf20d4b94e389744ffde70ae784debf216326d58c7dd0d79401e, uid : network namespace for sandbox "20919150d2fddf20d4b94e389744ffde70ae784debf216326d58c7dd0d79401e" is closed
            // E0515 11:47:23.973797   23508 portforward.go:362] error creating forwarding stream for port 8443 -> 8443: Timeout occurred
            // E0515 11:47:24.066781   23508 portforward.go:362] error creating forwarding stream for port 8443 -> 8443: Timeout occurred
            // E0515 11:47:37.957485   23508 portforward.go:340] error creating error stream for port 8443 -> 8443: Timeout occurred
            Template::PortForward => {
                self.event_hooks.insert(
                    "pfw-template-hook-1".to_string(),
                    Hook {
                        event: Event::DetectSubstring {
                            contains: "error".to_string(),
                            stream: Stream::Any,
                        },
                        action: Action::Restart,
                    },
                );
                self.event_hooks.insert(
                    "pfw-template-hook-2".to_string(),
                    Hook {
                        event: Event::DetectSubstring {
                            contains: "aborted".to_string(),
                            stream: Stream::Any,
                        },
                        action: Action::Restart,
                    },
                );
                self.event_hooks.insert(
                    "pfw-template-hook-3".to_string(),
                    Hook {
                        event: Event::DetectSubstring {
                            contains: "connection lost".to_string(),
                            stream: Stream::Any,
                        },
                        action: Action::Restart,
                    },
                );
            }
        }
    }
}
