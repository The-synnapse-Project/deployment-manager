mod admin;
mod config;
mod deploy;
mod types;
mod webhook;

use config::load_repo_configs;
use rocket::{launch, routes, Build, Rocket};
use std::env;
use std::sync::{Arc, Mutex};
use types::RepoConfigs;

pub type SharedRepoConfigs = Arc<Mutex<RepoConfigs>>;

#[launch]
fn rocket() -> Rocket<Build> {
    dotenvy::dotenv().ok();

    // Load configs into shared, thread-safe state
    let repo_configs: SharedRepoConfigs = Arc::new(Mutex::new(load_repo_configs()));

    // Get port from environment or use default
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9786);

    println!("Webhook server will start on port {}", port);
    let config_keys: Vec<String> = {
        let configs = repo_configs.lock().unwrap();
        configs.keys().cloned().collect()
    };
    println!("Configured repositories: {}", config_keys.join(", "));

    rocket::build()
        .manage(repo_configs)
        .mount("/", routes![webhook::webhook_handler])
        .mount(
            "/admin",
            routes![
                admin::list_repos,
                admin::add_update_repo,
                admin::delete_repo
            ],
        )
        .configure(rocket::Config {
            port,
            address: "0.0.0.0".parse().unwrap(),
            ..rocket::Config::default()
        })
}
