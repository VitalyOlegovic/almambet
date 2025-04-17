pub mod spam_filter_settings;

use crate::spam_filter::spam_filter_settings::SpamFilterSettings;
use tokio::time::Duration;
use crate::{mail_reader::message::Message, settings::Config};
use crate::spam_filter::spam_filter_settings::load_spam_filter_settings;
use crate::mail_reader::imap::{move_email_with_authentication, fetch_messages_from_server, create_session};
use tokio_cron_scheduler::{Job, JobScheduler};
use log::{debug,info,error};
use regex::Regex;

fn match_string(string: &str, pattern: &str) -> bool {
    let regex = Regex::new(pattern).unwrap();
    let result = regex.is_match(string);
    debug!("String {} pattern {} result {}", &string[..50.min(string.len())], pattern, result.to_string());
    result
}

fn match_many_strings(string: &str, patterns: &Vec<String>) -> bool {
    patterns.iter().any(|pattern| match_string(string, pattern))
}

pub fn check_message_spam(message: &Message, spam_filter_settings: &SpamFilterSettings) -> bool {
    // Check "from" patterns
    let from_matches = match_many_strings(&message.from, &spam_filter_settings.from_regular_expressions);

    // Check "title" patterns if you have them
    let title_matches = match_many_strings(&message.subject, &spam_filter_settings.title_regular_expressions);

    // Check "body" patterns if you have them
    let body_matches = match_many_strings(
        &message.content.clone().unwrap().as_ref(), 
        &spam_filter_settings.body_regular_expressions
    );

    // Return true if any pattern matches (message is spam)
    from_matches || title_matches || body_matches
}

async fn spam_filter(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    info!("Spam filter running");
    let spam_filter_settings = load_spam_filter_settings().unwrap();
    let mut imap_session = create_session(config).await?;
    let messages = fetch_messages_from_server(& mut imap_session, 10).await.unwrap();
    for message in &messages{
        if check_message_spam(message, &spam_filter_settings){
            info!("The message {} is spam, trying to move it", message.subject);
            match &message.message_id {
                Some(id) => {
                    info!("The spam message id is {}", id);
                    let _ = move_email_with_authentication(&mut imap_session, id.to_string(), "INBOX", "Spam").await;
                }
                None => {
                    error!("Cannot move spam message to spam folder.")
                },
            } 
            
        }
    }

    // Be nice to the server and log out
    imap_session.logout().await?;

    Ok(())
}

pub async fn entrypoint(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let sched = JobScheduler::new().await?;
    
    // Clone settings for the closure
    let settings_clone = config.clone();
    
    // Add a job that runs every 5 seconds
    sched.add(
        Job::new_repeated_async(
            Duration::from_secs(config.spam_filter.interval_seconds.into()), 
            move |_uuid, _l| {
                let settings = settings_clone.clone();
                Box::pin(async move {
                    let _ = spam_filter(&settings).await;
                })
        })?
    ).await?;

    // Start the scheduler
    tokio::spawn(async move {
        if let Err(e) = sched.start().await {
            eprintln!("Scheduler error: {}", e);
        }
    });

    Ok(())
}