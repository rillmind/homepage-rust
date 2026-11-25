mod auth;
mod casaos;
mod error;
mod files;
mod pc_info;
mod services;
mod storage;

use axum::{Router, middleware};
use std::{env, io, path::PathBuf};
use tokio::{
  net::TcpListener,
  signal::unix::{SignalKind, signal},
};

#[tokio::main]
async fn main() -> io::Result<()> {
  // Só local por padrão: o acesso de fora passa pelo nginx do front.
  let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:2306".into());
  let data = PathBuf::from(env::var("DATA_DIR").unwrap_or_else(|_| "data".into()));
  let files_root = env::var("FILES_ROOT").unwrap_or_else(|_| "/".into());
  let read_only = env::var("READ_ONLY_PATHS").unwrap_or_else(|_| "/DATA/AppData".into());

  let root = files::Root::new(files_root.into(), &read_only);
  let auth = auth::Auth::open(data.join("auth.json"))?;

  let app = Router::new()
    .merge(pc_info::routes())
    .merge(services::routes(services::Store::open(
      data.join("services.json"),
    )?))
    .merge(casaos::routes(root.clone()))
    .merge(files::routes(root, &data)?)
    .layer(middleware::from_fn_with_state(auth.clone(), auth::require))
    .merge(auth::routes(auth))
    .layer(middleware::from_fn(auth::same_origin));

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
