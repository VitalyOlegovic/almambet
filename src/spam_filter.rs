mod spam_filter_settings;

use spam_filter_settings::SpamFilterSettings;
use tokio::time::Duration;
use crate::{mail_reader::message::Message, settings::Settings};
use crate::spam_filter::spam_filter_settings::load_spam_filter_settings;
use crate::mail_reader::imap::move_email_with_authentication;
use crate::mail_reader::imap::fetch_messages_from_server;
use tokio_cron_scheduler::{Job, JobScheduler};
use log::{info,error};
use regex::Regex;


fn check_message_spam(message: &Message, spam_filter_settings: &SpamFilterSettings) -> bool {
    // Check "from" patterns
    let from_matches = spam_filter_settings.from_regular_expressions
        .iter()
        .any(|pattern| Regex::new(pattern).map_or(false, |re| re.is_match(&message.from)));

    // Check "title" patterns if you have them
    let title_matches = spam_filter_settings.title_regular_expressions
        .iter()
        .any(|pattern| Regex::new(pattern).map_or(false, |re| re.is_match(&message.subject)));

    // Check "body" patterns if you have them
    let body_matches = spam_filter_settings.body_regular_expressions
        .iter()
        .any(|pattern| Regex::new(pattern).map_or(
            false, 
            |re| re.is_match(&message.content.clone().unwrap().clone().as_ref())));

    // Return true if any pattern matches (message is spam)
    from_matches || title_matches || body_matches
}

async fn spam_filter(settings: &Settings) {
    // TODO Implement your spam filter logic here
    info!("Spam filter running");
    let spam_filter_settings = load_spam_filter_settings().unwrap();
    let messages = fetch_messages_from_server(&settings, 10).await.unwrap();
    for message in &messages{
        if check_message_spam(message, &spam_filter_settings){
            match &message.message_id {
                Some(id) => {
                    let _ = move_email_with_authentication(settings, id.to_string(), "INBOX", "Spam").await;
                }
                None => {
                    error!("Cannot move spam message to spam folder.")
                },
            } 
            
        }
    }
}

pub async fn entrypoint(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let sched = JobScheduler::new().await?;
    
    // Clone settings for the closure
    let settings_clone = settings.clone();
    
    // Add a job that runs every 5 seconds
    sched.add(
        Job::new_repeated_async(
            Duration::from_secs(settings.spam_filter_interval_seconds.into()), 
            move |_uuid, _l| {
                let settings = settings_clone.clone();
                Box::pin(async move {
                    spam_filter(&settings).await;
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