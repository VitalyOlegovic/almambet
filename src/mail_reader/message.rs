use anyhow::{bail, Result};
use mailparse::{parse_mail, MailHeaderMap};
use serde::{Serialize, Deserialize};
use log::warn;

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

        // Log unknown content types
        if content_type != "text/plain" && !content_type.starts_with("multipart/") {
            warn!("Unknown content type: {}", content_type);
        }

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

pub fn process_message(message: &async_imap::types::Fetch) -> Result<Message> {
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