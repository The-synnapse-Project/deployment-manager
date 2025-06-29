use crate::types::RepoConfigs;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CONFIG_FILE: &str = "repo-config.json";

fn config_path() -> PathBuf {
    // Use executable dir or current dir
    std::env::current_dir().unwrap().join(CONFIG_FILE)
}

pub fn load_repo_configs() -> RepoConfigs {
    let path = config_path();
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(err) => {
                eprintln!("Error loading repo config: {}", err);
                HashMap::new()
            }
        }
    } else {
        HashMap::new()
    }
}

pub fn save_repo_configs(configs: &RepoConfigs) -> bool {
    let path = config_path();
    match serde_json::to_string_pretty(&configs) {
        Ok(data) => match fs::write(&path, data) {
            Ok(()) => true,
            Err(err) => {
                eprintln!("Error saving repo config: {}", err);
                false
            }
        },
        Err(err) => {
            eprintln!("Error serializing repo config: {}", err);
            false
        }
    }
}
