use axum::{response::Html, routing::get, Router, Extension, response::Redirect};
use tera::Tera;
use std::sync::Arc;
use crate::mail_reader::message::Message;
use crate::mail_reader::imap::fetch_messages_from_server;
use crate::mail_reader::settings::Settings;
use log::{info, error};
use urlencoding;
use std::error::Error as StdError;

type AppError = Box<dyn StdError + Send + Sync>;

async fn render_error(tera: Arc<Tera>, error_message: String) -> Html<String> {
    let mut ctx = tera::Context::new();
    ctx.insert("error_message", &error_message);
    match tera.render("error.html", &ctx) {
        Ok(html) => Html(html),
        Err(e) => Html(format!("Error rendering error page: {}", e))
    }
}

async fn render_messages_page(
    messages: Arc<Vec<Message>>,
    tera: Arc<Tera>,
) -> Result<Html<String>, AppError> {
    let mut ctx = tera::Context::new();
    ctx.insert("messages", &*messages);
    let html = tera.render("emails.html", &ctx)?;
    Ok(Html(html))
}

async fn render_email_detail(
    message_id: String,
    messages: Arc<Vec<Message>>,
    tera: Arc<Tera>,
) -> Result<Html<String>, AppError> {
    let message = messages.iter()
        .find(|m| m.message_id.as_ref() == Some(&message_id))
        .ok_or_else(|| "Message not found")?;
    
    let mut ctx = tera::Context::new();
    ctx.insert("message", message);
    let html = tera.render("email_detail.html", &ctx)?;
    Ok(Html(html))
}

async fn move_to_spam(
    message_id: String,
    messages: Arc<Vec<Message>>,
) -> Result<Redirect, AppError> {
    let _message = messages.iter()
        .find(|m| m.message_id.as_ref() == Some(&message_id))
        .ok_or_else(|| "Message not found")?;
    
    // TODO: Implement actual move to spam functionality
    info!("Moving message {} to spam folder", message_id);
    
    Ok(Redirect::to("/"))
}

fn create_router(
    messages: Arc<Vec<Message>>,
    tera: Arc<Tera>,
) -> Router {
    let messages_for_list = messages.clone();
    let tera_for_list = tera.clone();
    let messages_for_detail = messages.clone();
    let tera_for_detail = tera.clone();
    let messages_for_spam = messages.clone();
    let tera_for_error = tera.clone();
    
    Router::new()
        .route("/", get(move || async move {
            match render_messages_page(messages_for_list.clone(), tera_for_list.clone()).await {
                Ok(html) => html,
                Err(e) => render_error(tera_for_list.clone(), format!("Error loading messages: {}", e)).await
            }
        }))
        .route("/email/:message_id", get(move |axum::extract::Path(message_id)| async move {
            match render_email_detail(message_id, messages_for_detail.clone(), tera_for_detail.clone()).await {
                Ok(html) => html,
                Err(e) => render_error(tera_for_detail.clone(), format!("Error loading email: {}", e)).await
            }
        }))
        .route("/email/:message_id/spam", get(move |axum::extract::Path(message_id)| async move {
            match move_to_spam(message_id, messages_for_spam.clone()).await {
                Ok(redirect) => redirect,
                Err(e) => Redirect::to(&format!("/error?message={}", urlencoding::encode(&format!("Error moving to spam: {}", e))))
            }
        }))
        .route("/error", get(move |axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>| async move {
            let error_message = params.get("message").cloned().unwrap_or_else(|| "Unknown error".to_string());
            render_error(tera_for_error.clone(), error_message).await
        }))
        .layer(Extension(tera.clone()))
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