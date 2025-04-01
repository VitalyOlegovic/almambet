use crate::mail_reader::message::Message;

pub fn display_messages(messages: &[Message]) {
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