mod mail_reader;
mod web;
mod web_services;
mod mail_move_rules;
mod settings;
mod tests;
use std::error::Error as StdError;

use clap::{Arg, ArgAction, Command};

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] {}: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::fs::File::create("output.log")?)
        .chain(std::io::stdout()) // optionally log to stdout too
        .apply()?;
    Ok(())
}

fn cli() -> Command {
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
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    setup_logger().expect("Failed to initialize logger");

    let config = settings::load_settings().unwrap();

    let matches = cli().get_matches();
    
    let once = matches.get_flag("once");
    let periodic = matches.get_flag("periodic");
    let web = matches.get_flag("web");
    let rest = matches.get_flag("rest");
    let spam = matches.get_flag("spam");
    
    if once{
        let _ = mail_move_rules::apply_rules(&config).await;
    }

    if periodic{
        let _ = mail_move_rules::entrypoint(&config).await; 
    }

    if web{
        let _ = web::entrypoint(&config).await;
    }

    if rest{
        let _ = web_services::entrypoint(&config).await;
    }

    if spam{
        let _ = mail_move_rules::delete_spam(&config).await;
    }

    Ok(())
}