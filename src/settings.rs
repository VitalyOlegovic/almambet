use serde::Deserialize;

use std::fs::File;
use std::io::BufReader;
use serde_yaml::Error;
use backtrace::Backtrace;
use log::error;

// Main configuration struct
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

// REST server configuration
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

pub fn load_settings() -> Result<Config, Error> {
    // Open the YAML file
    let file = File::open("src/resources/settings.yaml");
    let file = match file {
        Ok(file) => file,
        Err(err) => {
            error!("Error: {}", err);
        
            // Capture and print the backtrace
            let backtrace = Backtrace::new();
            error!("Backtrace:\n{:?}", backtrace);
            panic!("Cannot find settings")
        }
    };

    let reader = BufReader::new(file);

    // Parse the YAML file into the Settings struct
    let config_result= serde_yaml::from_reader(reader);
    let config: Config = match config_result {
        Ok(config) => config,
        Err(err) => {
            error!("Error: {}", err);
        
            // Capture and print the backtrace
            let backtrace = Backtrace::new();
            error!("Backtrace:\n{:?}", backtrace);
            panic!("Cannot deserialize settings")
        }
    };

    Ok(config)
}