use axum::{
  Json,
  http::StatusCode,
  response::{IntoResponse, Response},
};
use serde_json::json;
use std::io;

pub enum AppError {
  Invalid(&'static str),
  NotFound,
  Io(io::Error),
}

impl From<io::Error> for AppError {
  fn from(err: io::Error) -> Self {
    Self::Io(err)
  }
}

impl IntoResponse for AppError {
  fn into_response(self) -> Response {
    let (status, message) = match self {
      Self::Invalid(message) => (StatusCode::UNPROCESSABLE_ENTITY, message),
      Self::NotFound => (StatusCode::NOT_FOUND, "Serviço não encontrado"),
      Self::Io(err) => {
        eprintln!("Erro de E/S: {err}");
        (
          StatusCode::INTERNAL_SERVER_ERROR,
          "Erro interno do servidor",
        )
      }
    };
    (status, Json(json!({ "error": message }))).into_response()
  }
}
