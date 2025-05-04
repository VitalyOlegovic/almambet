pub mod mail_move_settings;

use crate::mail_move_rules::mail_move_settings::*;
use tokio::time::Duration;
use crate::{mail_reader::message::Message, settings::Config};
use crate::mail_move_rules::mail_move_settings::load_mail_move_config;
use crate::mail_reader::imap::{move_email_with_authentication, fetch_messages, create_session};
use tokio_cron_scheduler::{Job, JobScheduler};
use log::{debug,info,error};
use regex::Regex;
use itertools::Itertools;

fn match_string(string: &str, pattern: &str) -> bool {
    let regex = Regex::new(pattern).unwrap();
    let result = regex.is_match(string);
    let sanitized_string: String = string
    .chars()          // Iterate over characters (not bytes)
    .take(50)         // Take first 50 characters
    .filter(|c| *c != '\r' && *c != '\n')  // Filter out newlines
    .collect();       // Collect into a String
    debug!("String {} pattern {} result {}", sanitized_string, pattern, result);
    result
}

fn match_many_strings(string: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| match_string(string, pattern))
}

pub fn check_message_matches(message: &Message, rule: &Rule) -> bool {
    // Check "from" patterns
    let from_matches = match &rule.from {
        Some(patterns) => match_many_strings(&message.from, patterns),
        None => false,
    };

    // Check "title" patterns if you have them
    let title_matches = match &rule.title{
        Some(patterns) => match_many_strings(&message.subject, patterns),
        None => false,
    };

    // Check "body" patterns if you have them
    let body_matches = match &rule.body{
        Some(patterns) => match_many_strings(
            message.content.clone().unwrap().as_ref(), 
            patterns
        ),
        None => false,
    };

    // Return true if any pattern matches
    from_matches || title_matches || body_matches
}

pub async fn apply_rules(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    info!("Rule application running");
    let rules_config = load_mail_move_config()?;
    let mut imap_session = create_session(config).await?;
    
    let messages = fetch_messages(
        &mut imap_session, 
        "INBOX", 
        rules_config.messages_to_check
    ).await?;

    let matching_messages_and_rules: Vec<_> = messages
        .iter()
        .cartesian_product(&rules_config.rules)
        .filter(|(message, rule_wrapper)| check_message_matches(message, &rule_wrapper.rule))
        .collect();

    for (message, rule_wrapper) in matching_messages_and_rules {
        info!("The message {:?} is matching, trying to move it", message.subject);
        
        let Some(id) = &message.message_id else {
            error!("Cannot move message to another folder: missing message ID");
            continue;
        };
    
        info!("The matching message id is {}", id);
        if let Err(e) = move_email_with_authentication(
            &mut imap_session, 
            id.to_string(), 
            "INBOX", 
            &rule_wrapper.rule.target_folder
        ).await {
            error!("Failed to move message: {}", e);
        }
    }

    imap_session.logout().await?;
    Ok(())
}

pub async fn entrypoint(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let sched = JobScheduler::new().await?;
    
    // Clone settings for the closure
    let config_clone = config.clone();
    
    // Add a job that runs every N seconds
    sched.add(
        Job::new_repeated_async(
            Duration::from_secs(config.mail_mover.interval_seconds), 
            move |_uuid, _l| {
                let second_config_clone = config_clone.clone();
                Box::pin(async move {
                    let _ = apply_rules(&second_config_clone).await;
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