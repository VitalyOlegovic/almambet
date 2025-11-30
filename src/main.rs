mod mail_reader;
mod web;
mod web_services;
mod mail_move_rules;
mod settings;
mod tests;

use std::error::Error as StdError;
use clap::{Arg, ArgAction, Command};
use log::{error, info};

type AppResult<T> = Result<T, Box<dyn StdError>>;

/// Configuration for logger settings
#[derive(Debug)]
struct LoggerConfig {
    log_to_file: bool,
    log_to_stdout: bool,
    log_level: log::LevelFilter,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            log_to_file: true,
            log_to_stdout: true,
            log_level: log::LevelFilter::Debug,
        }
    }
}

/// Initialize logger with configuration
fn setup_logger(config: LoggerConfig) -> Result<(), fern::InitError> {
    let mut dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] {}: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(config.log_level);

    if config.log_to_file {
        dispatch = dispatch.chain(std::fs::File::create("output.log")?);
    }

    if config.log_to_stdout {
        dispatch = dispatch.chain(std::io::stdout());
    }

    dispatch.apply()?;
    Ok(())
}

/// Application operation modes
#[derive(Debug, Clone, Copy, PartialEq)]
enum OperationMode {
    Once,
    Periodic,
    Web,
    Rest,
    Spam,
}

impl OperationMode {
    fn from_cli_matches(matches: &clap::ArgMatches) -> Vec<Self> {
        let mut modes = Vec::new();

        if matches.get_flag("once") {
            modes.push(OperationMode::Once);
        }
        if matches.get_flag("periodic") {
            modes.push(OperationMode::Periodic);
        }
        if matches.get_flag("web") {
            modes.push(OperationMode::Web);
        }
        if matches.get_flag("rest") {
            modes.push(OperationMode::Rest);
        }
        if matches.get_flag("spam") {
            modes.push(OperationMode::Spam);
        }

        modes
    }
}

/// Build CLI command structure
fn build_cli() -> Command {
    Command::new("Email Rules Processor")
        .version("1.0")
        .about("Applies movement rules to email messages")
        .arg(
            Arg::new("once")
                .short('o')
                .long("once")
                .action(ArgAction::SetTrue)
                .help("Apply movement rules once and exit"),
        )
        .arg(
            Arg::new("periodic")
                .short('p')
                .long("periodic")
                .action(ArgAction::SetTrue)
                .help("Apply movement rules periodically"),
        )
        .arg(
            Arg::new("web")
                .short('w')
                .long("web")
                .action(ArgAction::SetTrue)
                .help("Run as a web application"),
        )
        .arg(
            Arg::new("rest")
                .short('r')
                .long("rest")
                .action(ArgAction::SetTrue)
                .help("Run a REST API endpoint"),
        )
        .arg(
            Arg::new("spam")
                .short('s')
                .long("spam")
                .action(ArgAction::SetTrue)
                .help("Deletes spam messages"),
        )
        .after_help(
            "Note: Multiple modes can be specified. If no mode is specified, \
            the application will run in 'once' mode by default."
        )
}

/// Execute the requested operation mode
async fn execute_mode(mode: OperationMode, config: &settings::Config) -> AppResult<()> {
    info!("Executing operation mode: {:?}", mode);
    
    match mode {
        OperationMode::Once => {
            mail_move_rules::apply_rules(config).await?;
            info!("Successfully executed once mode");
        }
        OperationMode::Periodic => {
            mail_move_rules::entrypoint(config).await?;
            info!("Successfully started periodic mode");
        }
        OperationMode::Web => {
            web::entrypoint(config).await?;
            info!("Successfully started web mode");
        }
        OperationMode::Rest => {
            web_services::entrypoint(config).await?;
            info!("Successfully started REST mode");
        }
        OperationMode::Spam => {
            mail_move_rules::delete_spam(config).await?;
            info!("Successfully executed spam deletion");
        }
    }
    
    Ok(())
}

/// Validate that the selected operation modes are compatible
fn validate_modes(modes: &[OperationMode]) -> AppResult<()> {
    if modes.is_empty() {
        return Ok(());
    }

    // Check for incompatible modes
    let has_server_mode = modes.contains(&OperationMode::Web) || modes.contains(&OperationMode::Rest);
    let has_processing_mode = modes.contains(&OperationMode::Once) || 
                             modes.contains(&OperationMode::Periodic) || 
                             modes.contains(&OperationMode::Spam);

    // If both server and processing modes are selected, warn the user
    if has_server_mode && has_processing_mode {
        log::warn!(
            "Both server modes (web/rest) and processing modes (once/periodic/spam) are selected. \
            This may lead to unexpected behavior."
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() -> AppResult<()> {
    // Initialize logger first
    setup_logger(LoggerConfig::default()).expect("Failed to initialize logger");
    
    info!("Starting Email Rules Processor");

    // Load configuration
    let config = settings::load_settings()
        .map_err(|e| {
            error!("Failed to load settings: {}", e);
            e
        })?;
    info!("Configuration loaded successfully");

    // Parse command line arguments
    let matches = build_cli().get_matches();
    let modes = OperationMode::from_cli_matches(&matches);

    // If no modes specified, default to 'once'
    let modes = if modes.is_empty() {
        info!("No operation mode specified, defaulting to 'once'");
        vec![OperationMode::Once]
    } else {
        modes
    };

    // Validate selected modes
    validate_modes(&modes)?;

    // Execute all requested modes
    for mode in modes {
        if let Err(e) = execute_mode(mode, &config).await {
            error!("Failed to execute mode {:?}: {}", mode, e);
            return Err(e);
        }
    }

    info!("Email Rules Processor completed successfully");
    Ok(())
}