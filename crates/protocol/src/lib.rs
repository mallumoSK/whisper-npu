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

/// Strips trailing hallucinated "you" / "You" occurrences from transcription text.
/// If the text consists solely of "you" (with optional punctuation/spaces) or ends with "you",
/// the trailing "you" token (and its preceding separator/attached punctuation) is removed.
pub fn strip_trailing_you(input: &str) -> String {
    let mut current = input.trim().to_string();

    loop {
        if current.is_empty() {
            break;
        }

        // 1. Check if the entire string consists solely of "you" or "thank you" (with punctuation)
        let alpha_only: String = current.chars().filter(|c| c.is_alphabetic()).collect();
        if alpha_only.eq_ignore_ascii_case("you") || alpha_only.eq_ignore_ascii_case("thankyou") {
            return String::new();
        }

        // 2. Find the last word in the string
        let last_alpha_idx = match current.char_indices().rfind(|(_, c)| c.is_alphabetic()) {
            Some((idx, _)) => idx,
            None => {
                // No alphabetic characters at all (e.g. "..." or "!?")
                return String::new();
            }
        };

        let before_end = &current[..=last_alpha_idx];
        let word_start = before_end
            .char_indices()
            .rfind(|(_, c)| !c.is_alphabetic())
            .map(|(idx, ch)| idx + ch.len_utf8())
            .unwrap_or(0);

        let last_word = &current[word_start..=last_alpha_idx];

        if last_word.eq_ignore_ascii_case("you") {
            // Cut off from word_start onwards
            let remainder = current[..word_start].trim_end();
            // Clean any trailing comma or dangling hyphen/colon/whitespace left behind
            let cleaned = remainder.trim_end_matches(|c: char| {
                c == ',' || c == '-' || c == ':' || c == ';' || c == ' ' || c == '\t' || c == '\n'
            });
            current = cleaned.to_string();
        } else {
            break;
        }
    }

    current
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

    #[test]
    fn test_strip_trailing_you() {
        assert_eq!(strip_trailing_you(""), "");
        assert_eq!(strip_trailing_you("you"), "");
        assert_eq!(strip_trailing_you("You"), "");
        assert_eq!(strip_trailing_you("you."), "");
        assert_eq!(strip_trailing_you("You."), "");
        assert_eq!(strip_trailing_you("you..."), "");
        assert_eq!(strip_trailing_you("  You!  "), "");
        assert_eq!(strip_trailing_you("Thank you."), "");
        assert_eq!(strip_trailing_you("thank you"), "");
        assert_eq!(strip_trailing_you("Hello you"), "Hello");
        assert_eq!(strip_trailing_you("Hello you."), "Hello");
        assert_eq!(strip_trailing_you("Hello, you."), "Hello");
        assert_eq!(strip_trailing_you("Hello world. You."), "Hello world.");
        assert_eq!(strip_trailing_you("Line 1\nLine 2 you"), "Line 1\nLine 2");
        assert_eq!(strip_trailing_you("you you"), "");
        assert_eq!(strip_trailing_you("What do you think?"), "What do you think?");
    }
}
