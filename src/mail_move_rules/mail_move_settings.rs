use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use log::error;
use anyhow::{anyhow, Result};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Rule {
    pub target_folder: String,
    pub from: Option<Vec<String>>,
    pub title: Option<Vec<String>>,
    pub body: Option<Vec<String>>,
    pub user_agent: Option<Vec<String>>,
    pub to: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct RuleWrapper {
    pub rule: Rule,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct RulesConfig {
    pub messages_to_check: u32,
    pub rules: Vec<RuleWrapper>,
}

pub fn load_mail_move_config() -> Result<RulesConfig> {
    // Find the config file in multiple locations
    let config_path = find_mail_move_config_file().ok_or_else(|| {
        let msg = "Could not find email_move_rules.yaml in any of the expected locations";
        error!("{}", msg);
        anyhow!(msg)
    })?;

    // Open the file
    let file = File::open(&config_path)
        .map_err(|err| {
            error!("Error opening config file at {:?}: {}", config_path, err);
            anyhow!("Cannot open config file: {}", err)
        })?;

    let reader = BufReader::new(file);

    // Parse the YAML file into the RulesConfig struct
    match yaml_serde::from_reader(reader) {
        Ok(config) => Ok(config),
        Err(err) => {
            error!("Error parsing config file at {:?}: {}", config_path, err);
            Err(anyhow!("Failed to parse YAML: {}", err))
        }
    }
}

fn find_mail_move_config_file() -> Option<PathBuf> {
    let mut possible_paths: Vec<PathBuf> = Vec::new();
    
    // 1. Current working directory
    possible_paths.push(PathBuf::from("email_move_rules.yaml"));
    possible_paths.push(PathBuf::from("email_move_rules.yml"));
    
    // 2. Config directory on Linux/Unix
    if let Some(config_dir) = dirs::config_dir() {
        possible_paths.push(config_dir.join("myapp").join("email_move_rules.yaml"));
        possible_paths.push(config_dir.join("myapp").join("email_move_rules.yml"));
    }
    
    // 3. Config directory on Windows
    if let Some(config_dir) = dirs::config_dir() {
        possible_paths.push(config_dir.join("MyApp").join("email_move_rules.yaml"));
        possible_paths.push(config_dir.join("MyApp").join("email_move_rules.yml"));
    }
    
    // 4. Same directory as executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            possible_paths.push(exe_dir.join("email_move_rules.yaml"));
            possible_paths.push(exe_dir.join("email_move_rules.yml"));
        }
    }
    
    // 5. /etc directory on Linux/Unix
    possible_paths.push(PathBuf::from("/etc/myapp/email_move_rules.yaml"));
    possible_paths.push(PathBuf::from("/etc/myapp/email_move_rules.yml"));
    
    // 6. Project source directory (development fallback)
    possible_paths.push(PathBuf::from("src/resources/email_move_rules.yaml"));
    possible_paths.push(PathBuf::from("src/resources/email_move_rules.yml"));

    // Find the first existing file
    possible_paths
        .into_iter()
        .find(|path| path.exists() && path.is_file())
}