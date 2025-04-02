use crate::mail_reader::message::Message;
use log::{info, error};

pub fn display_messages(messages: &[Message]) {
    messages
        .iter()
        .for_each(|message| {
            match serde_json::to_string_pretty(message) {
                Ok(json) => info!("{}", json),
                Err(e) => error!("Error converting to JSON: {}", e),
            }
            info!("---");
        });
} 