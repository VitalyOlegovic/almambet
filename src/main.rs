mod mail_reader;

use std::error::Error;
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
async fn main() -> Result<(), Box<dyn Error>> {
    setup_logger().expect("Failed to initialize logger");

    let settings = mail_reader::settings::load_settings()?;
    mail_reader::main(settings).await
}