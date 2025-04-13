use tokio::time::Duration;
use crate::mail_reader::settings::Settings;
use tokio_cron_scheduler::{Job, JobScheduler};
use log::info;

async fn spam_filter(_settings: &Settings) {
    // TODO Implement your spam filter logic here
    info!("Spam filter running")
}

pub async fn entrypoint(settings: Settings) -> Result<(), Box<dyn std::error::Error>> {
    let sched = JobScheduler::new().await?;
    
    // Clone settings for the closure
    let settings_clone = settings.clone();
    
    // Add a job that runs every 5 seconds
    sched.add(
        Job::new_repeated_async(Duration::from_secs(5), move |_uuid, _l| {
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