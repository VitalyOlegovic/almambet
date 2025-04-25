mod mail_reader;
mod web;
mod web_services;
mod mail_move_rules;
mod settings;
mod tests;
use std::error::Error as StdError;
use fern;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    setup_logger().expect("Failed to initialize logger");

    let config = settings::load_settings().unwrap();
    
    let _ = mail_move_rules::entrypoint(&config).await; 
    let _ = web::entrypoint(&config).await; // TODO: Remove
    let _ = web_services::entrypoint(&config).await;
    Ok(())
}