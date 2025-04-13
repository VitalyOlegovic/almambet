mod mail_reader;
mod web;
mod web_services;
mod spam_filter;
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

    let settings = mail_reader::settings::load_settings().unwrap();
    
    let _ = spam_filter::entrypoint(settings.clone()).await;
    let _ = web::entrypoint(settings.clone()).await;
    let _ = web_services::entrypoint(settings).await;
    Ok(())
}