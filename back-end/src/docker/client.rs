use crate::error::AppError;
use axum::{
  body::Bytes,
  http::{Method, Request, StatusCode, header},
};
use http_body_util::{BodyExt, Full, Limited};
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use serde_json::Value;
use std::{io::ErrorKind, path::PathBuf, time::Duration};
use tokio::{net::UnixStream, time::timeout};

const MAX_BODY: usize = 8 * 1024 * 1024;
/// Parar um contêiner pode levar o tempo de tolerância dele.
const TIMEOUT: Duration = Duration::from_secs(120);
const PULL_TIMEOUT: Duration = Duration::from_secs(600);

fn fail(message: impl Into<String>) -> AppError {
  AppError::Docker(message.into(), String::new())
}

/// API do Docker pelo socket. O podman fala a mesma língua.
#[derive(Clone)]
pub struct Client {
  pub socket: PathBuf,
}

impl Client {
  async fn send(
    &self,
    method: Method,
    path: &str,
    body: Option<&Value>,
  ) -> Result<(StatusCode, Bytes), AppError> {
    let stream = UnixStream::connect(&self.socket)
      .await
      .map_err(|err| match err.kind() {
        ErrorKind::PermissionDenied => {
          fail("Sem permissão para usar o Docker: o app precisa rodar como root")
        }
        ErrorKind::NotFound => fail("O socket do Docker não foi encontrado nesta máquina"),
        ErrorKind::ConnectionRefused => fail("O Docker não está rodando"),
        _ => fail(format!("Não foi possível falar com o Docker: {err}")),
      })?;
    let broken = |err: hyper::Error| fail(format!("Falha ao falar com o Docker: {err}"));

    let (mut sender, connection) = http1::handshake(TokioIo::new(stream))
      .await
      .map_err(broken)?;
    let connection = tokio::spawn(connection);
    let request = Request::builder()
      .method(method)
      .uri(path)
      .header(header::HOST, "docker")
      .header(header::CONTENT_TYPE, "application/json")
      .body(Full::new(Bytes::from(
        body.map(Value::to_string).unwrap_or_default(),
      )))
      .map_err(|err| fail(format!("Pedido inválido ao Docker: {err}")))?;
    let response = sender.send_request(request).await.map_err(broken)?;
    let status = response.status();
    let body = Limited::new(response.into_body(), MAX_BODY)
      .collect()
      .await
      .map_err(|err| fail(format!("Falha ao ler a resposta do Docker: {err}")))?
      .to_bytes();
    connection.abort();

    if status.is_client_error() || status.is_server_error() {
      let message = serde_json::from_slice::<Value>(&body)
        .ok()
        .and_then(|json| Some(json["message"].as_str()?.to_string()))
        .unwrap_or_else(|| format!("erro {status}"));
      return Err(fail(format!("O Docker recusou: {message}")));
    }
    Ok((status, body))
  }

  async fn request(
    &self,
    method: Method,
    path: &str,
    body: Option<&Value>,
    limit: Duration,
  ) -> Result<(StatusCode, Bytes), AppError> {
    timeout(limit, self.send(method, path, body))
      .await
      .unwrap_or_else(|_| Err(fail("O Docker demorou demais para responder")))
  }

  pub async fn get(&self, path: &str) -> Result<Value, AppError> {
    let (_, body) = self.request(Method::GET, path, None, TIMEOUT).await?;
    serde_json::from_slice(&body).map_err(|_| fail("Resposta inesperada do Docker"))
  }

  pub async fn post(&self, path: &str) -> Result<(), AppError> {
    self
      .request(Method::POST, path, None, TIMEOUT)
      .await
      .map(|_| ())
  }

  /// Cria um contêiner e devolve o id.
  pub async fn create(&self, config: &Value) -> Result<String, AppError> {
    let (_, body) = self
      .request(Method::POST, "/containers/create", Some(config), TIMEOUT)
      .await?;
    serde_json::from_slice::<Value>(&body)
      .ok()
      .and_then(|json| Some(json["Id"].as_str()?.to_string()))
      .ok_or_else(|| fail("Resposta inesperada do Docker"))
  }

  pub async fn remove(&self, id: &str) {
    let path = format!("/containers/{id}?force=1");
    let _ = self.request(Method::DELETE, &path, None, TIMEOUT).await;
  }

  /// Baixa a imagem se ela ainda não existir.
  pub async fn ensure_image(&self, image: &str, tag: &str) -> Result<(), AppError> {
    if self
      .get(&format!("/images/{image}:{tag}/json"))
      .await
      .is_ok()
    {
      return Ok(());
    }
    let path = format!("/images/create?fromImage={image}&tag={tag}");
    let (_, body) = self
      .request(Method::POST, &path, None, PULL_TIMEOUT)
      .await?;
    // A resposta já começou como 200 quando o download falha: o erro vem no meio dela.
    let error = String::from_utf8_lossy(&body)
      .lines()
      .filter_map(|line| serde_json::from_str::<Value>(line).ok())
      .find_map(|json| Some(json["error"].as_str()?.to_string()));
    match error {
      Some(error) => Err(fail(format!("Falha ao baixar {image}:{tag}: {error}"))),
      None => Ok(()),
    }
  }

  /// Últimas linhas do log. Sem log disponível, devolve vazio.
  pub async fn logs(&self, id: &str, since: i64) -> String {
    let path = format!("/containers/{id}/logs?stdout=1&stderr=1&tail=40&since={since}");
    match self.request(Method::GET, &path, None, TIMEOUT).await {
      Ok((_, body)) => clean(&demux(&body)),
      Err(_) => String::new(),
    }
  }
}

/// Sem TTY, o Docker põe antes de cada trecho 8 bytes: canal, três zeros e o tamanho.
fn demux(body: &[u8]) -> String {
  let mut text = Vec::new();
  let mut rest = body;
  while let [0..=2, 0, 0, 0, a, b, c, d, tail @ ..] = rest {
    let size = u32::from_be_bytes([*a, *b, *c, *d]) as usize;
    let size = size.min(tail.len());
    text.extend_from_slice(&tail[..size]);
    rest = &tail[size..];
  }
  text.extend_from_slice(rest);
  String::from_utf8_lossy(&text).into_owned()
}

/// Tira as cores do terminal e outros caracteres de controle.
fn clean(text: &str) -> String {
  let mut result = String::new();
  let mut chars = text.chars();
  while let Some(char) = chars.next() {
    if char == '\u{1b}' {
      if chars.next() == Some('[') {
        chars.by_ref().find(|c| ('@'..='~').contains(c));
      }
    } else if char == '\n' || char == '\t' || !char.is_control() {
      result.push(char);
    }
  }
  result.trim().to_string()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn separa_os_trechos_do_log() {
    let body = b"\x01\0\0\0\0\0\0\x03oi\n\x02\0\0\0\0\0\0\x04erro";
    assert_eq!(demux(body), "oi\nerro");
    assert_eq!(demux(b"com tty\n"), "com tty\n");
    assert_eq!(demux(b"\x01\0\0\0\0\0\0\xffcortado"), "cortado");
  }

  #[test]
  fn limpa_cores_do_log() {
    assert_eq!(clean("\x1b[31merro\x1b[0m\r\nfim"), "erro\nfim");
  }
}
