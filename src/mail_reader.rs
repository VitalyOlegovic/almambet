use serde::{Serialize, Deserialize};

pub mod encryption;
pub mod message;
pub mod imap;

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