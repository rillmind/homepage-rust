use axum::{
  Json,
  http::StatusCode,
  response::{IntoResponse, Response},
};
use serde_json::json;
use std::io::{self, ErrorKind};

#[derive(Debug)]
pub enum AppError {
  Invalid(&'static str),
  Unauthorized(&'static str),
  Forbidden(&'static str),
  Conflict(&'static str),
  TooMany(&'static str),
  NotFound,
  /// Mensagem e, se houver, os detalhes (o log do contêiner).
  Docker(String, String),
  Io(io::Error),
}

impl From<io::Error> for AppError {
  fn from(err: io::Error) -> Self {
    Self::Io(err)
  }
}

impl AppError {
  fn parts(&self) -> (StatusCode, &'static str) {
    match self {
      Self::Invalid(message) => (StatusCode::UNPROCESSABLE_ENTITY, message),
      Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message),
      Self::Forbidden(message) => (StatusCode::FORBIDDEN, message),
      Self::Conflict(message) => (StatusCode::CONFLICT, message),
      Self::TooMany(message) => (StatusCode::TOO_MANY_REQUESTS, message),
      Self::NotFound => (StatusCode::NOT_FOUND, "Não encontrado"),
      Self::Docker(..) => (StatusCode::BAD_GATEWAY, "Erro no Docker"),
      Self::Io(err) => match err.kind() {
        ErrorKind::NotFound => (StatusCode::NOT_FOUND, "Não encontrado"),
        ErrorKind::PermissionDenied => (StatusCode::FORBIDDEN, "Sem permissão"),
        ErrorKind::ReadOnlyFilesystem => (StatusCode::FORBIDDEN, "Disco somente leitura"),
        ErrorKind::AlreadyExists => (StatusCode::CONFLICT, "Já existe um item com esse nome"),
        ErrorKind::NotADirectory => (StatusCode::UNPROCESSABLE_ENTITY, "Não é uma pasta"),
        ErrorKind::ResourceBusy => (StatusCode::CONFLICT, "O item está em uso pelo sistema"),
        ErrorKind::StorageFull | ErrorKind::QuotaExceeded => {
          (StatusCode::INSUFFICIENT_STORAGE, "Sem espaço no disco")
        }
        ErrorKind::Interrupted => (StatusCode::CONFLICT, "Cancelado"),
        ErrorKind::InvalidInput => (StatusCode::UNPROCESSABLE_ENTITY, "Operação inválida"),
        _ => (
          StatusCode::INTERNAL_SERVER_ERROR,
          "Erro interno do servidor",
        ),
      },
    }
  }

  pub fn message(&self) -> &'static str {
    self.parts().1
  }
}

impl IntoResponse for AppError {
  fn into_response(self) -> Response {
    let (status, message) = self.parts();
    if let Self::Docker(message, details) = self {
      return (
        status,
        Json(json!({ "error": message, "details": details })),
      )
        .into_response();
    }
    if status == StatusCode::INTERNAL_SERVER_ERROR {
      eprintln!("{self:?}");
    }
    (status, Json(json!({ "error": message }))).into_response()
  }
}

/// Roda trabalho bloqueante (disco, hash de senha) fora das threads do servidor.
pub async fn blocking<T: Send + 'static>(
  work: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
  tokio::task::spawn_blocking(work)
    .await
    .unwrap_or_else(|err| Err(AppError::Io(io::Error::other(err))))
}
