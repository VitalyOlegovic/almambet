pub mod mail_move_settings;

use crate::mail_move_rules::mail_move_settings::*;
use tokio::time::Duration;
use crate::{mail_reader::message::Message, settings::Config};
use crate::mail_move_rules::mail_move_settings::load_mail_move_config;
use crate::mail_reader::imap::{create_session, delete_email_with_authentication, fetch_messages, move_email_with_authentication};
use tokio_cron_scheduler::{Job, JobScheduler};
use log::{debug,info,error};
use regex::Regex;
use itertools::Itertools;

fn sanitize_for_display(s: &str, max_chars: usize) -> String {
    s.chars()
        .take(max_chars)
        .filter(|c| !c.is_control() || *c == '\t')
        .map(|c| if c.is_whitespace() && c != ' ' { ' ' } else { c })
        .collect()
}

fn match_string(string: &str, pattern: &str) -> bool {
    // Compute result first (main logic)
    let result = Regex::new(pattern)
        .map(|regex| regex.is_match(string))
        .unwrap_or(false);
    
    // Logging with sanitization
    if log::log_enabled!(log::Level::Debug) {
        let sanitized = sanitize_for_display(string, 50);
        if result {
            debug!("match_string: input='{}' pattern='{}' result={}", 
                sanitized, pattern, result);
        }
    }
    
    result
}

fn match_many_strings(string: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| match_string(string, pattern))
}

pub fn check_message_matches(message: &Message, rule: &Rule) -> bool {
    // Early return if rule has no patterns to check (all fields are None)
    if rule.from.is_none() && rule.title.is_none() && rule.body.is_none() && rule.user_agent.is_none() {
        return false;
    }

    // Check each pattern type, short-circuiting as soon as we find a match
    if let Some(patterns) = &rule.from {
        if match_many_strings(&message.from, patterns) {
            return true;
        }
    }

    if let Some(patterns) = &rule.title {
        if match_many_strings(&message.subject, patterns) {
            return true;
        }
    }

    if let Some(patterns) = &rule.body {
        if let Some(content) = &message.content {
            if match_many_strings(content, patterns) {
                return true;
            }
        }
    }

    if let Some(patterns) = &rule.user_agent {
        if let Some(user_agent) = &message.user_agent {
            if match_many_strings(user_agent, patterns) {
                return true;
            }
        }
    }

    false
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

pub async fn delete_spam(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    info!("Deleting spam");
    let rules_config = load_mail_move_config()?;
    let mut imap_session = create_session(config).await?;
    
    let messages = fetch_messages(
        &mut imap_session, 
        "Spam", 
        rules_config.messages_to_check
    ).await?;

    for message in messages {
        info!("The message {:#?} is spam, trying to delete it", message);
        
        let Some(id) = &message.message_id else {
            error!("Cannot move message to another folder: missing message ID");
            continue;
        };
    
        info!("The matching message id is {}", id);
        if let Err(e) = delete_email_with_authentication(
            &mut imap_session, 
            id.to_string(), 
            "Spam"
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