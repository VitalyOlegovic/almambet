use anyhow::{Result,Error};
use async_imap::{Client, Session};
use futures::TryStreamExt;
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use std::error::Error as StdError;

use crate::mail_reader::message::Message;
use crate::settings::Config;
use crate::mail_reader::encryption;
use log::{debug, info, error};

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

// Fetch and process messages from the given mailbox
async fn fetch_messages(
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
    
    let successful_results: Vec<Message> = messages
        .iter()
        .filter_map(|message| crate::mail_reader::message::process_message(message).ok())
        .collect();
    
    Ok(successful_results)
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

pub async fn fetch_messages_from_server(imap_session: &mut ImapSession, count: u32) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
    
    
    // Fetch messages
    let messages = fetch_messages(imap_session, "INBOX", count).await?;
    
    // Be nice to the server and log out
    //imap_session.logout().await?;
    
    Ok(messages)
} 