use axum::{Router, routing::get};
use crate::mail_reader::settings::Settings;
use crate::mail_reader::imap::fetch_messages_from_server;
use std::error::Error;
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

async fn get_data(settings: Settings) -> Result<Json<String>, AppError> {
    let emails = fetch_messages_from_server(settings)
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

pub async fn entrypoint(settings: Settings) -> Result<(), Box<dyn Error>> {

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(move || get_data(settings.clone())));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
