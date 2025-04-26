use axum::{Router, routing::get};
use crate::settings::Config;
use crate::mail_reader::imap::fetch_messages_from_server;
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use std::fmt;

// Assuming you have some error type that implements std::error::Error
#[derive(Debug)]
struct AppError {
    message: String,
}

impl std::error::Error for AppError {}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error: {}", self.message),
        ).into_response()
    }
}

async fn get_data(folder: String, config: Config) -> Result<Json<String>, AppError> {

    let emails = fetch_messages_from_server(&config, &folder, 10)
        .await
        .map_err(|e| AppError {
            message: e.to_string(),
        })?;

    let json = serde_json::to_string(&emails)
        .map_err(|e| AppError {
            message: e.to_string(),
        })?;

    Ok(Json(json))
}

pub async fn entrypoint(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    // Clone settings once at the start instead of multiple times
    let settings_clone = config.clone();
    
    // Build our application with a route
    let app = Router::new()
        .route("/api/v1/emails", get(move || get_data("MAILBOX".to_string(), settings_clone)));

    // Run our app with hyper
    let addr = format!(
        "{}:{}", 
        config.server.host, 
        config.server.port
    );
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
