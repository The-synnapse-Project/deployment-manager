use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub path: String,
    pub secret: String,
    pub branch: Option<String>,
}

pub type RepoConfigs = HashMap<String, RepoConfig>;

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub repository: Option<Repository>,
    pub r#ref: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub full_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminRequest {
    #[serde(rename = "repoName")]
    pub repo_name: String,
    pub path: String,
    pub secret: String,
    pub branch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteRequest {
    #[serde(rename = "repoName")]
    pub repo_name: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}
