use axum::{response::Html, routing::get, Router, Extension};
use tera::Tera;
use std::sync::Arc;
use crate::mail_reader::message::Message;
use crate::mail_reader::imap::fetch_messages_from_server;
use crate::mail_reader::settings::Settings;
use log::{info, error};


async fn render_messages_page(
    messages: Arc<Vec<Message>>,
    tera: Arc<Tera>,
) -> Result<Html<String>, Box<dyn std::error::Error>> {
    let mut ctx = tera::Context::new();
    ctx.insert("messages", &*messages);
    let html = tera.render("emails.html", &ctx)?;
    Ok(Html(html))
}

async fn render_email_detail(
    message_id: String,
    messages: Arc<Vec<Message>>,
    tera: Arc<Tera>,
) -> Result<Html<String>, Box<dyn std::error::Error>> {
    let message = messages.iter()
        .find(|m| m.message_id.as_ref() == Some(&message_id))
        .ok_or_else(|| "Message not found")?;
    
    let mut ctx = tera::Context::new();
    ctx.insert("message", message);
    let html = tera.render("email_detail.html", &ctx)?;
    Ok(Html(html))
}

fn create_router(
    messages: Arc<Vec<Message>>,
    tera: Arc<Tera>,
) -> Router {
    let messages_for_list = messages.clone();
    let tera_for_list = tera.clone();
    let messages_for_detail = messages.clone();
    let tera_for_detail = tera.clone();
    
    Router::new()
        .route("/", get(move || async move {
            match render_messages_page(messages_for_list.clone(), tera_for_list.clone()).await {
                Ok(html) => html,
                Err(e) => Html(format!("Error rendering page: {}", e))
            }
        }))
        .route("/email/:message_id", get(move |axum::extract::Path(message_id)| async move {
            match render_email_detail(message_id, messages_for_detail.clone(), tera_for_detail.clone()).await {
                Ok(html) => html,
                Err(e) => Html(format!("Error rendering page: {}", e))
            }
        }))
        .layer(Extension(tera))
}

async fn start_server(router: Router) -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Server running on http://localhost:3000");
    axum::serve(listener, router).await?;
    Ok(())
}

pub async fn start_web_server(messages: Vec<Message>) -> Result<(), Box<dyn std::error::Error>> {
    let tera = Arc::new(Tera::new("templates/**/*.html")?);
    let messages = Arc::new(messages);
    
    let router = create_router(Arc::clone(&messages), Arc::clone(&tera));
    start_server(router).await
}

pub async fn entrypoint(settings: Settings) -> Result<(), Box<dyn std::error::Error>> {
    let result = fetch_messages_from_server(settings).await;
    match result {
        Ok(messages) => {
            if let Err(e) = start_web_server(messages).await {
                error!("Error starting web server: {}", e);
                return Err(e);
            }else{
                info!("Web server started");
                return Ok(());
            }
        }
        Err(e) => {
            error!("Error fetching messages: {}", e);
            return Err(e);
        }
    }
}