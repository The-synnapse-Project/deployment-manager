use crate::config::save_repo_configs;
use crate::types::{AdminRequest, ApiError, ApiResponse, DeleteRequest, RepoConfig};
use crate::SharedRepoConfigs;
use rocket::{
    delete, get,
    http::Status,
    post,
    request::{self, FromRequest, Request},
    serde::json::Json,
    State,
};

// Admin token guard
pub struct AdminToken;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminToken {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let admin_token = std::env::var("ADMIN_TOKEN").unwrap_or_default();
        if admin_token.is_empty() {
            return request::Outcome::Error((Status::InternalServerError, ()));
        }

        match request.headers().get_one("x-admin-token") {
            Some(token) if token == admin_token => request::Outcome::Success(AdminToken),
            _ => request::Outcome::Error((Status::Unauthorized, ())),
        }
    }
}

/// List all repositories
#[get("/repos")]
pub fn list_repos(
    _admin: AdminToken,
    configs: &State<SharedRepoConfigs>,
) -> Json<std::collections::HashMap<String, RepoConfig>> {
    match configs.lock() {
        Ok(map) => Json(map.clone()),
        Err(_) => Json(std::collections::HashMap::new()),
    }
}

/// Add or update a repository configuration
#[post("/repos", data = "<request>")]
pub fn add_update_repo(
    _admin: AdminToken,
    configs: &State<SharedRepoConfigs>,
    request: Json<AdminRequest>,
) -> Result<Json<ApiResponse>, (Status, Json<ApiError>)> {
    let mut map = match configs.lock() {
        Ok(map) => map,
        Err(_) => {
            return Err((
                Status::InternalServerError,
                Json(ApiError {
                    error: "Failed to acquire lock on repository configurations".to_string(),
                }),
            ));
        }
    };

    map.insert(
        request.repo_name.clone(),
        RepoConfig {
            path: request.path.clone(),
            secret: request.secret.clone(),
            branch: request.branch.clone(),
        },
    );

    save_repo_configs(&map);

    Ok(Json(ApiResponse {
        success: true,
        message: format!("Repository {} configured successfully", request.repo_name),
    }))
}

/// Delete a repository configuration
#[delete("/repos", data = "<request>")]
pub fn delete_repo(
    _admin: AdminToken,
    configs: &State<SharedRepoConfigs>,
    request: Json<DeleteRequest>,
) -> Result<Json<ApiResponse>, (Status, Json<ApiError>)> {
    let mut map = match configs.lock() {
        Ok(map) => map,
        Err(_) => {
            return Err((
                Status::InternalServerError,
                Json(ApiError {
                    error: "Failed to acquire lock on repository configurations".to_string(),
                }),
            ));
        }
    };

    if map.remove(&request.repo_name).is_some() {
        save_repo_configs(&map);
        Ok(Json(ApiResponse {
            success: true,
            message: format!("Repository {} removed successfully", request.repo_name),
        }))
    } else {
        Err((
            Status::NotFound,
            Json(ApiError {
                error: format!("Repository {} not found", request.repo_name),
            }),
        ))
    }
}
