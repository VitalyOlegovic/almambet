use anyhow::{Result,Error};
use async_imap::{Client, Session};
use futures::TryStreamExt;
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use std::{cmp::Ordering, error::Error as StdError};
use chrono::DateTime;

use crate::mail_reader::message::Message;
use crate::settings::Config;
use crate::mail_reader::encryption;
use log::{debug, info, error};
use futures::StreamExt;  // For the stream's next() method

use super::message::fetch_to_message;

pub type ImapSession = Session<Compat<tokio_native_tls::TlsStream<tokio::net::TcpStream>>>;

// Establish a TLS-encrypted connection to the IMAP server
async fn connect_to_server(server: &str, port: u16) -> Result<tokio_native_tls::TlsStream<TcpStream>> {
    let imap_addr = (server, port);
    let tcp_stream = TcpStream::connect(imap_addr).await?;
    let tls = tokio_native_tls::TlsConnector::from(native_tls::TlsConnector::new()?);
    let tls_stream = tls.connect(server, tcp_stream).await?;
    
    info!("connected to {}:{}", server, port);
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
    
    info!("logged in as {}", username);
    Ok(imap_session)
}

// Calculate the range string for fetching the most recent messages
fn calculate_message_range(total_messages: u32, count: u32) -> String {
    let start = if total_messages > count { total_messages - count + 1 } else { 1 };
    format!("{}:{}", start, total_messages)
}

pub fn sort_messages_by_date_desc(messages: &mut Vec<Message>) {
    messages.sort_by(|a, b| {
        let a_date = DateTime::parse_from_rfc2822(&a.date).ok();
        let b_date = DateTime::parse_from_rfc2822(&b.date).ok();

        match (a_date, b_date) {
            (Some(a_dt), Some(b_dt)) => b_dt.cmp(&a_dt), // Descending order (newest first)
            (Some(_), None) => Ordering::Less, // Parsed dates come before unparsed
            (None, Some(_)) => Ordering::Greater,
            (None, None) => b.date.cmp(&a.date), // Fallback: String comparison (lexicographical)
        }
    });
}

// Fetch and process messages from the given mailbox
pub async fn fetch_messages(
    session: &mut Session<Compat<tokio_native_tls::TlsStream<TcpStream>>>,
    mailbox: &str,
    count: u32
) -> Result<Vec<Message>> {
    let mailbox_data = session.select(mailbox).await?;
    info!("{} selected", mailbox);
    
    let total_messages = mailbox_data.exists;
    let range = calculate_message_range(total_messages, count);
    
    // Fetch both headers and body
    let messages_stream = session.fetch(&range, "(RFC822 BODY.PEEK[])").await?;
    let messages: Vec<_> = messages_stream.try_collect().await?;
    
    let mut successful_results: Vec<Message> = messages
        .iter()
        .filter_map(|message| crate::mail_reader::message::fetch_to_message(message).ok())
        .collect();

    // Sort by date descending
    sort_messages_by_date_desc(&mut successful_results);
    
    Ok(successful_results)
}

pub async fn find_message_by_id(
    session: &mut Session<Compat<tokio_native_tls::TlsStream<TcpStream>>>,
    message_id: &str,
    source_mailbox: &str,
) -> Result<Option<Message>> {
    // First, select the source mailbox to ensure we're in the right folder
    session.select(source_mailbox).await?;

    // Search for the message by its Message-ID header
    let messages = session.search(format!("HEADER Message-ID {}", message_id)).await?;

    if messages.is_empty() {
        return Ok(None);
    }
    
    // Get the first message ID from the search results
    let first_message_id = messages.iter().next().expect("Non-empty messages but couldn't get first").to_string();
    
    // Fetch the first matching message
    let mut fetch_result = session.fetch(first_message_id, "RFC822").await?;
    
    // Extract the message body from the fetch result
    let option_result_fetch = fetch_result.next().await;

    match option_result_fetch {
        Some(result_fetch) => match result_fetch{
            Ok(fetch) => Ok(Some(fetch_to_message(&fetch).expect("Error"))),
            Err(_) => Ok(None),
        },
        None => Ok(None),
    }
}

pub async fn move_message_by_message_id(
    session: &mut Session<Compat<tokio_native_tls::TlsStream<TcpStream>>>,
    message_id: &str,
    source_mailbox: &str,
    target_mailbox: &str,
) -> Result<()> {
    debug!("move_message_by_message_id message_id {} source {} target {}", message_id, source_mailbox, target_mailbox);

    // First, select the INBOX to ensure we're in the right folder
    session.select(source_mailbox).await?;
    
    // Search for the message by its Message-ID header
    let search_result = session.search(format!("HEADER Message-ID {}", message_id)).await?;
    
    if search_result.is_empty() {
        return Err(anyhow::anyhow!("Message not found"));
    }
    
    // Get the UID of the message
    let uid_result = session.uid_search(format!("HEADER Message-ID {}", message_id)).await?;
    let uid = uid_result.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("No UID found"))?;
    
    // Move the message to the specified folder using UID
    session.uid_mv(uid.to_string(), target_mailbox).await?;
    info!("Moved message {} to {} folder", message_id, target_mailbox);
    
    Ok(())
}


pub async fn delete_message_by_message_id(
    session: &mut Session<Compat<tokio_native_tls::TlsStream<TcpStream>>>,
    message_id: &str,
    mailbox: &str,
) -> Result<()> {
    debug!("delete_message_by_message_id message_id {} mailbox {}", message_id, mailbox);

    // Select the mailbox to ensure we're in the right folder
    session.select(mailbox).await?;
    
    // Search for the message by its Message-ID header using UID search
    let uid_result = session.uid_search(format!("HEADER Message-ID {}", message_id)).await?;
    
    if uid_result.is_empty() {
        return Err(anyhow::anyhow!("Message with Message-ID '{}' not found in mailbox '{}'", message_id, mailbox));
    }
    
    // Get the UID of the message
    let uid = uid_result.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("No UID found for message with Message-ID '{}'", message_id))?;
    
    // Mark the message for deletion using UID store and consume the stream
    session
        .uid_store(uid.to_string(), "+FLAGS (\\Deleted)")
        .await?
        .try_collect::<Vec<_>>()
        .await?;
    
    // Permanently remove the message by expunging the mailbox and consume the stream
    session
        .expunge()
        .await?
        .try_collect::<Vec<_>>()
        .await?;
    
    info!("Deleted message {} from mailbox {}", message_id, mailbox);
    
    Ok(())
}

pub async fn move_email_with_authentication(
    imap_session: &mut ImapSession,
    message_id: String, 
    source_mailbox: &str, 
    target_mailbox: &str
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    debug!("move_email_with_authentication message_id {} source {} target {}", message_id, source_mailbox, target_mailbox);
    
    if let Err(e) = move_message_by_message_id(imap_session, &message_id, source_mailbox, target_mailbox).await {
        error!("Failed to move message: {}", e);
        return Err(e.into());
    }
    
    Ok(())
}

pub async fn delete_email_with_authentication(
    imap_session: &mut ImapSession,
    message_id: String, 
    source_mailbox: &str
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    debug!("delete_message_by_message_id message_id {} source {}", message_id, source_mailbox);
    
    if let Err(e) = delete_message_by_message_id(imap_session, &message_id, source_mailbox).await {
        error!("Failed to move message: {}", e);
        return Err(e.into());
    }
    
    Ok(())
}

pub async fn create_session(config: &Config) -> Result<ImapSession, Error>{
    // Get credentials
    let (username, password) = encryption::get_credentials(config.imap.username.as_str())?;
        
    // Connect to server
    let tls_stream = connect_to_server(config.imap.server.as_str(), config.imap.port).await?;
    let compat_stream = tls_stream.compat();
    let client = Client::new(compat_stream);

    // Log in
    let imap_session = login_to_server(client, &username, &password).await?;

    Ok(imap_session)
}

pub async fn fetch_messages_from_server(config: &Config, mailbox: &str, count: u32) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    
    let mut imap_session = create_session(config).await?;
    // Fetch messages
    let messages = fetch_messages(&mut imap_session, mailbox, count).await?;
    
    // Be nice to the server and log out
    imap_session.logout().await?;
    
    Ok(messages)
} 

pub async fn list_imap_folders(
    config: &Config
) -> Result<Vec<String>, Error> {

    let mut imap_session = create_session(config).await?;

    let folders = imap_session
        .list(Some(""), Some("*"))
        .await?
        // First convert Result to Option with ok()
        .map_ok(|name|name.name().to_string())
        .try_collect::<Vec<String>>()
        .await?;

    // Be nice to the server and log out
    imap_session.logout().await?;

    Ok(folders)
}