use axum::{response::Html, routing::get, Router, Extension, response::Redirect};
use tera::Tera;
use std::sync::Arc;
use crate::mail_reader::message::Message;
use crate::mail_reader::imap::{create_session, fetch_messages_from_server, find_message_by_id, move_email_with_authentication};
use crate::settings::Config;
use log::info;
use anyhow::Error;
type AppError = Error;

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
    config: &Config,
    message_id: String,
    folder_name: String,
    tera: Arc<Tera>,
) -> Result<Html<String>, AppError> {
    let mut imap_session = create_session(config).await?;
    let message = find_message_by_id(&mut imap_session, &message_id, &folder_name).await?.expect("Error");
    
    let mut ctx = tera::Context::new();
    ctx.insert("message", &message);
    let html = tera.render("email_detail.html", &ctx)?;
    Ok(Html(html))
}

async fn move_message(
    message_id: String,
    target_folder: String,
    config: &Config,
) -> Result<Redirect, AppError> {
    let mut imap_session = create_session(config).await?;
    let _ = move_email_with_authentication(&mut imap_session, message_id, "INBOX", &target_folder).await;

    Ok(Redirect::to("/"))
}

async fn start_server(router: Router) -> Result<(), AppError> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Server running on http://localhost:3000");
    axum::serve(listener, router).await?;
    Ok(())
}

async fn create_router(
    tera: Arc<Tera>,
    config: &Config,
) -> Router {
    let tera_for_list = tera.clone();
    let tera_for_detail = tera.clone();
    let tera_for_error = tera.clone();
    let settings_for_move_message = config.clone();
    let settings_for_spam = config.clone();
    let config_for_detail = config.clone();

    // TODO use the IMAP session holder here
    
    Router::new()
        .route("/", get(|| async { Redirect::permanent("/inbox/INBOX") }))
        .route("/inbox/:folder_name", get(move |axum::extract::Path(folder_name): axum::extract::Path<String>| async move {
            let messages = fetch_messages_from_server(&settings_for_spam.clone(), &folder_name, 10)
                .await
                .expect("Cannot fetch messages");
            match render_messages_page(Arc::new(messages.clone()), tera_for_list.clone()).await {
                Ok(html) => html,
                Err(e) => render_error(tera_for_list.clone(), format!("Error loading messages: {}", e)).await
            }
        }))
        .route("/email/:folder_name/:message_id", get(move |axum::extract::Path((folder_name, message_id))| async move {
            match render_email_detail(&config_for_detail.clone(), message_id, folder_name, tera_for_detail.clone()).await {
                Ok(html) => html,
                Err(e) => render_error(tera_for_detail.clone(), format!("Error loading email: {}", e)).await
            }
        }))
        .route("/email/:message_id/move/:target_folder", get(
            move |axum::extract::Path((message_id, target_folder)): axum::extract::Path<(String, String)>| async move {
                match move_message(message_id, target_folder, &settings_for_move_message.clone()).await {
                    Ok(redirect) => redirect,
                    Err(e) => Redirect::to(&format!("/error?message={}", urlencoding::encode(&format!("Error moving message: {}", e))))
                }
            }
        ))
        .route("/error", get(move |axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>| async move {
            let error_message = params.get("message").cloned().unwrap_or_else(|| "Unknown error".to_string());
            render_error(tera_for_error.clone(), error_message).await
        }))
        .layer(Extension(tera.clone()))
}

pub async fn start_web_server(config: &Config) -> Result<(), AppError> {
    let tera = Arc::new(Tera::new("templates/**/*.html")?);
    
    let router = create_router(Arc::clone(&tera), config).await;
    start_server(router).await
}

pub async fn entrypoint(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    start_web_server(config).await.map_err(Into::into)
}