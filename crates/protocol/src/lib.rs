use serde::{Deserialize, Serialize};

pub const DEFAULT_SOCKET_PATH: &str = "/run/whisper-npu/daemon.sock";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonState {
    Listening,
    Paused,
    Idle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientCommand {
    StartListening,
    PauseListening,
    ResumeListening,
    ClearBuffer,
    Stop,
    StopAndPaste {
        #[serde(default)]
        pid: u32,
        #[serde(default = "default_paste_delay")]
        delay_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wayland_display: Option<String>,
    },
    CancelAndExit {
        #[serde(default)]
        pid: u32,
    },
}

fn default_paste_delay() -> u64 {
    180
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonEvent {
    StateChanged { data: DaemonState },
    PartialTranscript { data: String },
    FinalTranscript { data: String },
    Error { data: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commands_json() {
        assert_eq!(
            serde_json::to_string(&ClientCommand::StartListening).unwrap(),
            r#"{"type":"StartListening"}"#
        );
        assert_eq!(
            serde_json::to_string(&ClientCommand::PauseListening).unwrap(),
            r#"{"type":"PauseListening"}"#
        );
        assert_eq!(
            serde_json::to_string(&ClientCommand::ResumeListening).unwrap(),
            r#"{"type":"ResumeListening"}"#
        );
        assert_eq!(
            serde_json::to_string(&ClientCommand::ClearBuffer).unwrap(),
            r#"{"type":"ClearBuffer"}"#
        );
        assert_eq!(
            serde_json::to_string(&ClientCommand::Stop).unwrap(),
            r#"{"type":"Stop"}"#
        );
        assert_eq!(
            serde_json::to_string(&ClientCommand::StopAndPaste {
                pid: 1234,
                delay_ms: 180,
                text: None,
                wayland_display: None,
            })
            .unwrap(),
            r#"{"type":"StopAndPaste","pid":1234,"delay_ms":180}"#
        );

        let cmd: ClientCommand = serde_json::from_str(r#"{"type":"StopAndPaste"}"#).unwrap();
        assert_eq!(
            cmd,
            ClientCommand::StopAndPaste {
                pid: 0,
                delay_ms: 180,
                text: None,
                wayland_display: None,
            }
        );

        let cmd: ClientCommand = serde_json::from_str(r#"{"type":"StartListening"}"#).unwrap();
        assert_eq!(cmd, ClientCommand::StartListening);
    }

    #[test]
    fn test_events_json() {
        assert_eq!(
            serde_json::to_string(&DaemonEvent::StateChanged {
                data: DaemonState::Listening
            })
            .unwrap(),
            r#"{"type":"StateChanged","data":"Listening"}"#
        );

        let event_json = r#"{"type":"PartialTranscript","data":"hello world"}"#;
        let event: DaemonEvent = serde_json::from_str(event_json).unwrap();
        assert_eq!(
            event,
            DaemonEvent::PartialTranscript {
                data: "hello world".to_string()
            }
        );

        let event_json = r#"{"type":"FinalTranscript","data":"sentence ended."}"#;
        let event: DaemonEvent = serde_json::from_str(event_json).unwrap();
        assert_eq!(
            event,
            DaemonEvent::FinalTranscript {
                data: "sentence ended.".to_string()
            }
        );

        let event_json = r#"{"type":"Error","data":"microphone disconnected"}"#;
        let event: DaemonEvent = serde_json::from_str(event_json).unwrap();
        assert_eq!(
            event,
            DaemonEvent::Error {
                data: "microphone disconnected".to_string()
            }
        );
    }
}
