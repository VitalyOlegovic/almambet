use serde::{Deserialize,Serialize};

use std::fs::File;
use std::io::BufReader;
use serde_yaml::Error;
use backtrace::Backtrace;
use log::error;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Rule {
    pub target_folder: String,
    pub from: Option<Vec<String>>,
    pub title: Option<Vec<String>>,
    pub body: Option<Vec<String>>,
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

pub fn load_mail_move_config() -> Result<RulesConfig, Error> {
    let file = File::open("src/resources/email_move_rules.yaml");
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
    let parsing_result= serde_yaml::from_reader(reader);
    let rules_config: RulesConfig = match parsing_result {
        Ok(settings) => settings,
        Err(err) => {
            error!("Error: {}", err);
        
            // Capture and print the backtrace
            let backtrace = Backtrace::new();
            error!("Backtrace:\n{:?}", backtrace);
            panic!("Cannot deserialize message filter settings")
        }
    };

    Ok(rules_config)
}