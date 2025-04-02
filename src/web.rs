use axum::{response::Html, routing::get, Router, Extension};
use tera::Tera;
use serde::Serialize;
use chrono::Local;
use std::sync::Arc;
use mail_reader::{Message, fetch_messages_from_server};
use settings::Settings;
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

fn create_router(
    messages: Arc<Vec<Message>>,
    tera: Arc<Tera>,
) -> Router {
    Router::new()
        .route("/", {
            let messages = Arc::clone(&messages);
            let tera = Arc::clone(&tera);
            get(move || async move {
                render_messages_page(messages, tera).await
            })
        })
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

#[tokio::main]
async fn main() {
    let settings = Settings::new().expect("Failed to load settings");
    match fetch_messages_from_server(settings).await {
        Ok(messages) => {
            if let Err(e) = start_web_server(messages).await {
                error!("Error starting web server: {}", e);
            }
        }
        Err(e) => {
            error!("Error fetching messages: {}", e);
        }
    }
}