mod error;
mod pc_info;
mod services;

use axum::Router;
use std::{env, io};
use tokio::{
  net::TcpListener,
  signal::unix::{SignalKind, signal},
};

#[tokio::main]
async fn main() -> io::Result<()> {
  // Só local por padrão: o acesso de fora passa pelo /api do front.
  let addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:2306".into());
  let data = env::var("DATA_PATH").unwrap_or_else(|_| "data/services.json".into());

  let app = Router::new()
    .merge(pc_info::routes())
    .merge(services::routes(services::Store::open(data.into())?));

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
