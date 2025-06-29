use crate::types::{NotificationPayload, RepoConfig};
use std::env;
use std::process::Stdio;
use tokio::process::Command;

/// Deploys a repository based on the config, notifies Discord on failure/success.
pub async fn deploy_repository(
    repo_config: RepoConfig,
    repo_name: String,
) -> Result<String, String> {
    let repo_path = &repo_config.path;

    // Create the deployment command
    let commands = [
        format!("cd {}", repo_path),
        "git pull".to_string(),
        "cd ..".to_string(),
        "docker compose up -d --build".to_string(),
        "sleep 10 && docker compose ps --filter \"status=running\" | grep -v \"Down\" || (echo \"Deployment verification failed\" && exit 1)".to_string(),
    ];

    let full_command = commands.join(" && ");

    println!("Executing deployment command: {}", full_command);

    // Execute the command
    let child = Command::new("sh")
        .arg("-c")
        .arg(&full_command)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("Failed to wait for command: {}", e))?;

    if output.status.success() {
        let success_msg = format!("Deployment successful for {}", repo_name);
        println!("{}", success_msg);

        // Send success notification
        if let Ok(webhook_url) = env::var("DISCORD_WEBHOOK_URL") {
            let notification = NotificationPayload {
                content: format!(
                    "✅ Deployment successful for {}\nPath: {}\nTimestamp: {}",
                    repo_name,
                    repo_config.path,
                    chrono::Utc::now().to_rfc3339()
                ),
            };

            if let Err(e) = send_discord_notification(&webhook_url, &notification).await {
                eprintln!("Failed to send Discord notification: {}", e);
            }
        }

        Ok(success_msg)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let error_msg = format!(
            "Command failed with exit code: {:?}\nStdout: {}\nStderr: {}",
            output.status.code(),
            stdout,
            stderr
        );

        println!("Deployment error for {}: {}", repo_name, error_msg);

        // Send failure notification
        if let Ok(webhook_url) = env::var("DISCORD_WEBHOOK_URL") {
            let notification = NotificationPayload {
                content: format!(
                    "❌ Deployment failed for {}\nPath: {}\nExit code: {:?}\nStdOut:```{}```\nStdErr:```{}```",
                    repo_name,
                    repo_config.path,
                    output.status.code(),
                    stdout,
                    stderr
                ),
            };

            if let Err(e) = send_discord_notification(&webhook_url, &notification).await {
                eprintln!("Failed to send Discord notification: {}", e);
            }
        }

        Err(error_msg)
    }
}

async fn send_discord_notification(
    webhook_url: &str,
    notification: &NotificationPayload,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client
        .post(webhook_url)
        .json(notification)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "Discord webhook returned status: {}",
            response.status()
        ))
    }
}
