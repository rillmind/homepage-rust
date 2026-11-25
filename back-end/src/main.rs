mod auth;
mod casaos;
mod docker;
mod error;
mod files;
mod front;
mod pc_info;
mod services;
mod storage;

use axum::{Router, middleware};
use std::{env, io, path::PathBuf};
use tokio::{
  net::TcpListener,
  signal::unix::{SignalKind, signal},
};

/// Aceita porta ou ip:porta. Sem nada, só local, para o desenvolvimento.
fn listen_addr(listen: &str) -> String {
  match listen {
    "" => "127.0.0.1:2306".into(),
    port if port.parse::<u16>().is_ok() => format!("0.0.0.0:{port}"),
    addr => addr.into(),
  }
}

#[tokio::main]
async fn main() -> io::Result<()> {
  let addr = listen_addr(&env::var("LISTEN").unwrap_or_default());
  let data = PathBuf::from(env::var("DATA_DIR").unwrap_or_else(|_| "data".into()));
  let files_root = env::var("FILES_ROOT").unwrap_or_else(|_| "/".into());
  let read_only = env::var("READ_ONLY_PATHS").unwrap_or_else(|_| "/DATA/AppData".into());

  let root = files::Root::new(files_root.into(), &read_only);
  let auth = auth::Auth::open(data.join("auth.json"))?;

  let api = Router::new()
    .merge(pc_info::routes())
    .merge(services::routes(
      services::Store::open(data.join("services.json"))?,
      docker::Docker::new(root.clone()),
    ))
    .merge(casaos::routes(root.clone()))
    .merge(files::routes(root, &data)?)
    .layer(middleware::from_fn_with_state(auth.clone(), auth::require))
    .merge(auth::routes(auth))
    .layer(middleware::from_fn(auth::same_origin))
    .fallback(|| async { error::AppError::NotFound });

  let mut app = Router::new().nest("/homepage-api", api);
  // Em desenvolvimento quem serve o front é o Vite.
  if let Ok(dir) = env::var("FRONT_DIR") {
    app = app.merge(front::routes(dir));
  }

  let listener = TcpListener::bind(&addr).await?;
  println!("Servidor rodando em http://{addr}");

  let mut sigint = signal(SignalKind::interrupt())?;
  let mut sigterm = signal(SignalKind::terminate())?;

  // Para de aceitar conexões no sinal e espera as requisições ativas terminarem.
  axum::serve(listener, app)
    .with_graceful_shutdown(async move {
      tokio::select! {
        _ = sigint.recv() => {}
        _ = sigterm.recv() => {}
      }
      println!("Desligando servidor...");
    })
    .await
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn listen_aceita_porta_ou_endereco() {
    assert_eq!(listen_addr(""), "127.0.0.1:2306");
    assert_eq!(listen_addr("5173"), "0.0.0.0:5173");
    assert_eq!(listen_addr("127.0.0.1:80"), "127.0.0.1:80");
  }
}
