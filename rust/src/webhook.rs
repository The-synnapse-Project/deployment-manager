use crate::deploy::deploy_repository;
use crate::types::{ApiError, WebhookPayload};
use crate::SharedRepoConfigs;
use hmac::{Hmac, Mac};
use rocket::{
    data::{Data, ToByteUnit},
    http::Status,
    post,
    request::{FromRequest, Request},
    serde::json::Json,
    State,
};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

// Custom request guard to extract headers we need
pub struct WebhookHeaders {
    pub signature: Option<String>,
    pub event: Option<String>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for WebhookHeaders {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let signature = request
            .headers()
            .get_one("x-hub-signature-256")
            .map(|s| s.to_string());
        let event = request
            .headers()
            .get_one("x-github-event")
            .map(|s| s.to_string());

        rocket::request::Outcome::Success(WebhookHeaders { signature, event })
    }
}

/// Handler for POST /webhook
#[post("/webhook", data = "<data>")]
pub async fn webhook_handler(
    configs: &State<SharedRepoConfigs>,
    headers: WebhookHeaders,
    data: Data<'_>,
) -> Result<String, (Status, Json<ApiError>)> {
    // Read the raw body
    let body = match data.open(5.megabytes()).into_bytes().await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Err((
                Status::BadRequest,
                Json(ApiError {
                    error: "Failed to read request body".to_string(),
                }),
            ))
        }
    };

    let body_str = String::from_utf8_lossy(&body);

    // Parse JSON payload
    let payload: WebhookPayload = match serde_json::from_str(&body_str) {
        Ok(p) => p,
        Err(_) => {
            return Err((
                Status::BadRequest,
                Json(ApiError {
                    error: "Invalid JSON payload".to_string(),
                }),
            ))
        }
    };

    // Extract repository information
    let full_repo_name = match &payload.repository {
        Some(repo) => &repo.full_name,
        None => {
            return Err((
                Status::BadRequest,
                Json(ApiError {
                    error: "Repository not specified in payload".to_string(),
                }),
            ))
        }
    };

    // Find matching repository configuration
    let repo_config = {
        let configs_lock = configs.lock().unwrap();
        match configs_lock.get(full_repo_name) {
            Some(config) => config.clone(),
            None => {
                return Err((
                    Status::NotFound,
                    Json(ApiError {
                        error: format!("No configuration found for repository: {}", full_repo_name),
                    }),
                ))
            }
        }
    };

    // Verify the request is coming from GitHub
    let signature = match headers.signature {
        Some(sig) => sig,
        None => {
            return Err((
                Status::Unauthorized,
                Json(ApiError {
                    error: "No signature provided".to_string(),
                }),
            ))
        }
    };

    // Calculate expected signature using repo-specific secret
    let mut hmac = match HmacSha256::new_from_slice(repo_config.secret.as_bytes()) {
        Ok(h) => h,
        Err(_) => {
            return Err((
                Status::InternalServerError,
                Json(ApiError {
                    error: "Failed to create HMAC".to_string(),
                }),
            ))
        }
    };

    hmac.update(&body);
    let expected_signature = format!("sha256={}", hex::encode(hmac.finalize().into_bytes()));

    // Verify signatures match
    if signature != expected_signature {
        return Err((
            Status::Unauthorized,
            Json(ApiError {
                error: "Invalid signature".to_string(),
            }),
        ));
    }

    // Check if this is a push event
    let event = match headers.event {
        Some(e) => e,
        None => {
            return Err((
                Status::BadRequest,
                Json(ApiError {
                    error: "No GitHub event specified".to_string(),
                }),
            ))
        }
    };

    if event != "push" {
        println!("Ignoring event: {}", event);
        return Ok("OK".to_string());
    }

    // Check if this is the configured branch (if specified)
    if let Some(configured_branch) = &repo_config.branch {
        if let Some(ref_str) = &payload.r#ref {
            let branch = ref_str.strip_prefix("refs/heads/").unwrap_or(ref_str);
            if branch != configured_branch {
                println!(
                    "Ignoring push to branch {}, only deploying {}",
                    branch, configured_branch
                );
                return Ok(format!("OK: Ignored push to {}", branch));
            }
        }
    }

    println!(
        "Received valid webhook for {}, deploying...",
        full_repo_name
    );

    // Deploy the repository
    match deploy_repository(repo_config, full_repo_name.clone()).await {
        Ok(msg) => Ok(msg),
        Err(err) => Err((
            Status::InternalServerError,
            Json(ApiError {
                error: format!("Deployment failed for {}: {}", full_repo_name, err),
            }),
        )),
    }
}
