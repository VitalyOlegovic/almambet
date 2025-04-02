use serde::Deserialize;

use std::fs::File;
use std::io::BufReader;
use serde_yaml::Error;
use backtrace::Backtrace;
use log::{error};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub imap_server: String,
    pub port: u16,
    pub email_address: String,
}

pub fn load_settings() -> Result<Settings, Error> {
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
    let settings= serde_yaml::from_reader(reader);
    let settings: Settings = match settings {
        Ok(settings) => settings,
        Err(err) => {
            error!("Error: {}", err);
        
            // Capture and print the backtrace
            let backtrace = Backtrace::new();
            error!("Backtrace:\n{:?}", backtrace);
            panic!("Cannot deserialize settings")
        }
    };

    Ok(settings)
}