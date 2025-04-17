use serde::Deserialize;

use std::fs::File;
use std::io::BufReader;
use serde_yaml::Error;
use backtrace::Backtrace;
use log::error;

#[derive(Debug, Default, Deserialize, Clone)]
pub struct SpamFilterSettings {
    pub from_regular_expressions: Vec<String>,
    pub title_regular_expressions: Vec<String>,
    pub body_regular_expressions: Vec<String>,
}

pub fn load_spam_filter_settings() -> Result<SpamFilterSettings, Error> {
    let file = File::open("src/resources/spam_filter_settings.yaml");
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
    let settings: SpamFilterSettings = match settings {
        Ok(settings) => settings,
        Err(err) => {
            error!("Error: {}", err);
        
            // Capture and print the backtrace
            let backtrace = Backtrace::new();
            error!("Backtrace:\n{:?}", backtrace);
            panic!("Cannot deserialize spam filter settings")
        }
    };

    Ok(settings)
}