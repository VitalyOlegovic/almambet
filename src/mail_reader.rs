use anyhow::{bail, Result};
use async_imap::{types::Fetch, Client, Session};
use futures::TryStreamExt;
use mailparse::{parse_mail, MailHeaderMap};
use rpassword::prompt_password;
use settings::Settings;
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use std::error::Error;
use serde::{Serialize, Deserialize};

pub mod settings;

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub subject: String,
    pub from: String,
    pub date: String,
    pub to: Option<String>,
    pub cc: Option<String>,
    pub bcc: Option<String>,
    pub reply_to: Option<String>,
    pub message_id: Option<String>,
    pub content_type: Option<String>,
    pub content: Option<String>,
}

// Process a single email message to extract subject, from, and date
fn process_message(message: &Fetch) -> Result<Message> {
    let body = message.body().expect("message did not have a body!");
    let parsed_mail = parse_mail(body)?;
    let subject = parsed_mail.headers.get_first_value("Subject");
    let from = parsed_mail.headers.get_first_value("From");
    let date = parsed_mail.headers.get_first_value("Date");
    let to = parsed_mail.headers.get_first_value("To");
    let cc = parsed_mail.headers.get_first_value("Cc");
    let bcc = parsed_mail.headers.get_first_value("Bcc");
    let reply_to = parsed_mail.headers.get_first_value("Reply-To");
    let message_id = parsed_mail.headers.get_first_value("Message-ID");
    let content_type = parsed_mail.headers.get_first_value("Content-Type");

    // Extract content from the message body
    let content = if let Some(subparts) = parsed_mail.subparts.first() {
        // Try to get text content from the first subpart
        Some(subparts.get_body()?.to_string())
    } else {
        // If no subparts, try to get content from the main body
        Some(parsed_mail.get_body()?.to_string())
    };
    
    match (subject, from, date) {
        (Some(s), Some(f), Some(d)) => Ok(Message {
            subject: s,
            from: f,
            date: d,
            to,
            cc,
            bcc,
            reply_to,
            message_id,
            content_type,
            content,
        }),
        _ => bail!("Cannot parse the message"),
    }
}

// Establish a TLS-encrypted connection to the IMAP server
async fn connect_to_server(server: &str, port: u16) -> Result<tokio_native_tls::TlsStream<TcpStream>> {
    let imap_addr = (server, port);
    let tcp_stream = TcpStream::connect(imap_addr).await?;
    let tls = tokio_native_tls::TlsConnector::from(native_tls::TlsConnector::new()?);
    let tls_stream = tls.connect(server, tcp_stream).await?;
    
    println!("-- connected to {}:{}", server, port);
    Ok(tls_stream)
}

// Login to the IMAP server and return an authenticated session
async fn login_to_server(
    client: Client<Compat<tokio_native_tls::TlsStream<TcpStream>>>,
    username: &str, 
    password: &str
) -> Result<Session<Compat<tokio_native_tls::TlsStream<TcpStream>>>> {
    let imap_session = client
        .login(username, password)
        .await
        .map_err(|e| e.0)?;
    
    println!("-- logged in as {}", username);
    Ok(imap_session)
}

// Calculate the range string for fetching the most recent messages
fn calculate_message_range(total_messages: u32, count: u32) -> String {
    let start = if total_messages > count { total_messages - count + 1 } else { 1 };
    format!("{}:{}", start, total_messages)
}

// Fetch and process messages from the given mailbox
// Using a concrete type instead of a trait bound
async fn fetch_messages(
    session: &mut Session<Compat<tokio_native_tls::TlsStream<TcpStream>>>,
    mailbox: &str,
    count: u32
) -> Result<Vec<Message>> {
    let mailbox_data = session.select(mailbox).await?;
    println!("-- {} selected", mailbox);
    
    let total_messages = mailbox_data.exists;
    let range = calculate_message_range(total_messages, count);
    
    // Fetch both headers and body
    let messages_stream = session.fetch(&range, "(RFC822 BODY.PEEK[])").await?;
    let messages: Vec<_> = messages_stream.try_collect().await?;
    
    let successful_results: Vec<Message> = messages
        .iter()
        .filter_map(|message| process_message(message).ok())
        .collect();
    
    Ok(successful_results)
}

// Display the processed messages
fn display_messages(messages: &[Message]) {
    messages
        .iter()
        .for_each(|message| {
            match serde_json::to_string_pretty(message) {
                Ok(json) => println!("{}", json),
                Err(e) => println!("Error converting to JSON: {}", e),
            }
            println!("---");
        });
}

// Get user credentials
fn get_credentials(login: &str) -> Result<(String, String)> {
    let password = prompt_password("Enter your password: ")?;
    Ok((login.to_string(), password))
}

pub async fn main(mail_settings: Settings) -> Result<(), Box<dyn Error>> {
    
    // Get credentials
    let (username, password) = get_credentials(mail_settings.email_address.as_str())?;
    
    // Connect to server
    let tls_stream = connect_to_server(mail_settings.imap_server.as_str(), mail_settings.port).await?;
    let compat_stream = tls_stream.compat();
    let client = Client::new(compat_stream);
    
    // Log in
    let mut imap_session = login_to_server(client, &username, &password).await?;
    
    // Fetch messages
    let messages = fetch_messages(&mut imap_session, "INBOX", 10).await?;
    
    // Display results
    display_messages(&messages);
    
    // Be nice to the server and log out
    imap_session.logout().await?;
    
    Ok(())
}