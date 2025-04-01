use anyhow::{bail, Result};
use async_imap::{types::Fetch, Client, Session};
use futures::TryStreamExt;
use mailparse::{parse_mail, MailHeaderMap};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use std::error::Error;
use serde::{Serialize, Deserialize};

pub mod settings;
pub mod encryption;

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
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub content: Vec<u8>,
}

fn extract_attachments(parsed_mail: &mailparse::ParsedMail) -> Result<Vec<Attachment>> {
    let mut attachments = Vec::new();

    fn process_part(part: &mailparse::ParsedMail, attachments: &mut Vec<Attachment>) -> Result<()> {
        let content_type = part.headers.get_first_value("Content-Type")
            .unwrap_or_else(|| "text/plain".to_string());
        
        let content_disposition = part.headers.get_first_value("Content-Disposition")
            .unwrap_or_default();

        // Check if this is an attachment
        if content_disposition.to_lowercase().contains("attachment") {
            let filename = part.headers.get_first_value("Content-Disposition")
                .and_then(|disp| {
                    disp.split("filename=")
                        .nth(1)
                        .map(|f| f.trim_matches('"').to_string())
                })
                .unwrap_or_else(|| "unnamed_attachment".to_string());

            let content = part.get_body_raw()?;
            
            attachments.push(Attachment {
                filename,
                content_type,
                size: content.len(),
                content,
            });
        }

        // Recursively process subparts
        for subpart in &part.subparts {
            process_part(subpart, attachments)?;
        }

        Ok(())
    }

    process_part(parsed_mail, &mut attachments)?;
    Ok(attachments)
}

fn extract_text_content(parsed_mail: &mailparse::ParsedMail) -> Result<Option<String>> {
    fn find_text_part(part: &mailparse::ParsedMail) -> Result<Option<String>> {
        let content_type = part.headers.get_first_value("Content-Type")
            .unwrap_or_else(|| "text/plain".to_string());

        // If this is a text part, return its content
        if content_type.starts_with("text/") {
            return Ok(Some(part.get_body()?.to_string()));
        }

        // Recursively search subparts
        for subpart in &part.subparts {
            if let Some(text) = find_text_part(subpart)? {
                return Ok(Some(text));
            }
        }

        Ok(None)
    }

    find_text_part(parsed_mail)
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

    // Extract text content and attachments
    let content = extract_text_content(&parsed_mail)?;
    let attachments = extract_attachments(&parsed_mail)?;
    
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
            attachments,
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

pub async fn fetch_messages_from_server(mail_settings: settings::Settings) -> Result<Vec<Message>, Box<dyn Error>> {
    // Get credentials
    let (username, password) = encryption::get_credentials(mail_settings.email_address.as_str())?;
    
    // Connect to server
    let tls_stream = connect_to_server(mail_settings.imap_server.as_str(), mail_settings.port).await?;
    let compat_stream = tls_stream.compat();
    let client = Client::new(compat_stream);
    
    // Log in
    let mut imap_session = login_to_server(client, &username, &password).await?;
    
    // Fetch messages
    let messages = fetch_messages(&mut imap_session, "INBOX", 10).await?;
    
    // Be nice to the server and log out
    imap_session.logout().await?;
    
    Ok(messages)
}

pub async fn main(mail_settings: settings::Settings) -> Result<(), Box<dyn Error>> {
    let messages = fetch_messages_from_server(mail_settings).await?;
    display_messages(&messages);
    Ok(())
}