use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use yaml_serde::Error;
use log::error;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub imap: ImapConfig,
    pub mail_mover: MailMoverConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ImapConfig {
    pub server: String,
    pub port: u16,
    pub username: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MailMoverConfig {
    #[serde(rename = "check_interval")]
    pub interval_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

pub fn load_settings() -> Result<Config, Error> {
    let config_path = find_config_file().unwrap_or_else(|| {
        error!("Could not find settings.yaml in any of the expected locations");
        panic!("Cannot find settings.yaml");
    });

    let file = match File::open(&config_path) {
        Ok(file) => file,
        Err(err) => {
            error!("Error opening config file at {:?}: {}", config_path, err);
            panic!("Cannot open settings file: {}", err);
        }
    };

    let reader = BufReader::new(file);

    match yaml_serde::from_reader(reader) {
        Ok(config) => Ok(config),
        Err(err) => {
            error!("Error parsing config: {}", err);
            panic!("Cannot deserialize settings: {}", err);
        }
    }
}

fn find_config_file() -> Option<PathBuf> {
    // Build the list of possible paths, handling Options properly
    let mut possible_paths: Vec<PathBuf> = Vec::new();
    
    // 1. Current working directory
    possible_paths.push(PathBuf::from("settings.yaml"));
    
    // 2. Config directory on Unix/Linux
    if let Some(config_dir) = dirs::config_dir() {
        possible_paths.push(config_dir.join("myapp/settings.yaml"));
    }
    
    // 3. User's home config directory on Windows (same as above but different app name)
    if let Some(config_dir) = dirs::config_dir() {
        possible_paths.push(config_dir.join("MyApp").join("settings.yaml"));
    }
    
    // 4. Relative to the executable (for production deployments)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            possible_paths.push(exe_dir.join("settings.yaml"));
        }
    }
    
    // 5. Project source directory (development fallback)
    possible_paths.push(PathBuf::from("src/resources/settings.yaml"));

    // Find the first existing file
    for path in possible_paths {
        if path.exists() && path.is_file() {
            return Some(path);
        }
    }

    None
}