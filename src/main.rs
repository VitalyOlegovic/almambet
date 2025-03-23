mod mail_reader;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let settings = mail_reader::settings::load_settings()?;
    mail_reader::main(settings).await
}