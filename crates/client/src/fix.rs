use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

pub const DEFAULT_LLAMA_API_URL: &str = "http://127.0.0.1:8080/v1/chat/completions";
pub const DEFAULT_LLAMA_HEALTH_URL: &str = "http://127.0.0.1:8080/health";
pub const DEFAULT_LLAMA_SERVICE: &str = "llama.service";

#[derive(Debug, Clone)]
pub enum FixProgress {
    Status(String),
    Done(Result<String, String>),
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

pub async fn check_service_health(health_url: &str) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(1000))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    if let Ok(res) = client.get(health_url).send().await {
        if res.status().is_success() {
            return true;
        }
    }
    false
}

pub fn check_systemctl_active(service: &str) -> bool {
    let status = std::process::Command::new("systemctl")
        .args(["is-active", "--quiet", service])
        .status();
    if let Ok(s) = status {
        if s.success() {
            return true;
        }
    }

    let status_sudo = std::process::Command::new("sudo")
        .args(["systemctl", "is-active", "--quiet", service])
        .status();
    match status_sudo {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

pub fn start_systemctl_service(service: &str) -> Result<()> {
    info!("Starting {} via sudo systemctl start...", service);
    let res_sudo = std::process::Command::new("sudo")
        .args(["systemctl", "start", service])
        .status();

    if let Ok(s) = res_sudo {
        if s.success() {
            info!("Successfully started {} with sudo", service);
            return Ok(());
        }
    }

    // Fallback without sudo if sudo failed
    let res = std::process::Command::new("systemctl")
        .args(["start", service])
        .status();

    if let Ok(s) = res {
        if s.success() {
            return Ok(());
        }
    }

    anyhow::bail!("Failed to start {} via sudo systemctl start", service);
}

pub fn stop_systemctl_service(service: &str) -> Result<()> {
    info!("Stopping {} via sudo systemctl stop...", service);
    let res_sudo = std::process::Command::new("sudo")
        .args(["systemctl", "stop", service])
        .status();

    if let Ok(s) = res_sudo {
        if s.success() {
            info!("Successfully stopped {} with sudo", service);
            return Ok(());
        }
    }

    let res = std::process::Command::new("systemctl")
        .args(["stop", service])
        .status();

    if let Ok(s) = res {
        if s.success() {
            return Ok(());
        }
    }

    anyhow::bail!("Failed to stop {} via sudo systemctl stop", service);
}

pub async fn ensure_service_ready<F>(
    service_name: &str,
    health_url: &str,
    mut on_status: F,
) -> Result<()>
where
    F: FnMut(&str),
{
    // Step 1: Check if is alive right now (health)
    on_status("Checking LLM health...");
    if check_service_health(health_url).await {
        info!("LLM service is already healthy and responding");
        return Ok(());
    }

    // Step 2: Not healthy on HTTP - check systemctl status
    let is_active = check_systemctl_active(service_name);
    if !is_active {
        on_status(&format!("Starting {}...", service_name));
        info!("{} is inactive. Starting via systemctl...", service_name);
        start_systemctl_service(service_name)
            .with_context(|| format!("Failed to start service {}", service_name))?;
    } else {
        info!("{} is active in systemd, waiting for /health to become ready...", service_name);
        on_status("Waiting for LLM model to finish loading...");
    }

    // Step 3: Wait and poll health check until ready (up to 30 seconds)
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(30);
    while start.elapsed() < timeout {
        if check_service_health(health_url).await {
            info!("LLM service confirmed healthy and ready after {:?}", start.elapsed());
            return Ok(());
        }
        let elapsed = start.elapsed().as_secs();
        on_status(&format!("Loading LLM model ({}s)...", elapsed));
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    anyhow::bail!(
        "Timed out waiting for {} health check at {}",
        service_name,
        health_url
    );
}

pub async fn request_text_fix_with_progress<F>(
    service_name: &str,
    endpoint: &str,
    health_url: &str,
    original_text: &str,
    instruction: &str,
    mut on_status: F,
) -> Result<String>
where
    F: FnMut(&str),
{
    // 1. Ensure service is active and healthy
    ensure_service_ready(service_name, health_url, &mut on_status).await?;

    on_status("Polishing text with LLM...");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()?;

    let prompt = if instruction.trim().is_empty() {
        format!(
            "Original text:\n{}\n\nRevision instructions:\nFix grammar, spelling, punctuation, and capitalization. Maintain the original meaning. Return ONLY the revised text with no explanation, preface, or conversational filler.\n\nRevised text:",
            original_text
        )
    } else {
        format!(
            "Original text:\n\"\"\"\n{}\n\"\"\"\n\nUser instructions for revision:\n{}\n\nReturn ONLY the revised text following the instructions, with no conversational filler, preface, or markdown wrapping.",
            original_text,
            instruction.trim()
        )
    };

    let payload = ChatRequest {
        messages: vec![
            ChatMessage {
                role: "system",
                content: "You are an expert AI editor. Revise the provided text strictly according to the user instructions. Output only the revised text with no conversational explanation.",
            },
            ChatMessage {
                role: "user",
                content: &prompt,
            },
        ],
        temperature: 0.2,
    };

    info!("Sending text to LLM for revision at {}", endpoint);

    let res = client
        .post(endpoint)
        .json(&payload)
        .send()
        .await
        .context("Failed to connect to LLM server. Is llama.service running?")?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        anyhow::bail!("LLM server returned error status {}: {}", status, body);
    }

    let chat_res: ChatResponse = res
        .json()
        .await
        .context("Failed to parse LLM JSON response")?;

    let revised = chat_res
        .choices
        .first()
        .map(|c| c.message.content.trim())
        .unwrap_or("")
        .to_string();

    let clean_revised = clean_code_fences(&revised);
    Ok(clean_revised)
}

fn clean_code_fences(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with("```") && trimmed.ends_with("```") {
        let lines: Vec<&str> = trimmed.lines().collect();
        if lines.len() >= 2 {
            let inner = &lines[1..lines.len() - 1];
            return inner.join("\n").trim().to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_code_fences() {
        let text = "```\nHello world\n```";
        assert_eq!(clean_code_fences(text), "Hello world");

        let text_with_lang = "```markdown\nHello world\n```";
        assert_eq!(clean_code_fences(text_with_lang), "Hello world");

        let plain = "Hello world";
        assert_eq!(clean_code_fences(plain), "Hello world");
    }

    #[tokio::test]
    async fn test_llama_service_live_fix() {
        if check_service_health(DEFAULT_LLAMA_HEALTH_URL).await {
            let res = request_text_fix_with_progress(
                DEFAULT_LLAMA_SERVICE,
                DEFAULT_LLAMA_API_URL,
                DEFAULT_LLAMA_HEALTH_URL,
                "this are a bad english text with errrors",
                "Fix all spelling and grammatical errors.",
                |s| println!("Status: {}", s),
            )
            .await;
            assert!(res.is_ok(), "Llama request failed: {:?}", res.err());
            let fixed = res.unwrap();
            assert!(!fixed.is_empty());
            println!("Llama result: {}", fixed);
        } else {
            println!("Skipping live test: LLaMA service not healthy");
        }
    }
}
