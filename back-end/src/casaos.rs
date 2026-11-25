use crate::{
  error::{AppError, blocking},
  files::Root,
};
use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use serde_yaml_ng::Value;
use std::fs;

const APPS: &str = "/var/lib/casaos/apps";

#[derive(Debug, PartialEq, Serialize)]
struct App {
  name: String,
  icon: String,
  protocol: &'static str,
  port: u16,
  description: String,
}

pub fn routes(root: Root) -> Router {
  Router::new()
    .route("/casaos/apps", get(apps))
    .with_state(root)
}

async fn apps(State(root): State<Root>) -> Result<Json<Vec<App>>, AppError> {
  let apps = blocking(move || {
    let dir = match root.resolve(APPS) {
      Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
      dir => dir?,
    };
    let mut apps: Vec<App> = fs::read_dir(root.real(&dir))?
      .flatten()
      .filter_map(|entry| fs::read_to_string(entry.path().join("docker-compose.yml")).ok())
      .filter_map(|text| parse(&text))
      .collect();
    apps.sort_by_key(|app| app.name.to_lowercase());
    Ok(apps)
  })
  .await?;
  Ok(Json(apps))
}

/// Texto simples ou um mapa de idiomas.
fn text(value: &Value) -> String {
  let found = match value {
    Value::Mapping(_) => ["custom", "pt_br", "pt_BR", "en_us", "en_US"]
      .iter()
      .filter_map(|key| value.get(key)?.as_str())
      .find(|text| !text.is_empty()),
    other => other.as_str(),
  };
  found.unwrap_or_default().trim().to_string()
}

/// Só o bloco x-casaos sai daqui: o resto do compose pode ter senhas.
fn parse(yaml: &str) -> Option<App> {
  let compose: Value = serde_yaml_ng::from_str(yaml).ok()?;
  let info = compose.get("x-casaos")?;
  let field = |key: &str| info.get(key).map(text).unwrap_or_default();

  let port = match info.get("port_map")? {
    Value::Number(port) => u16::try_from(port.as_u64()?).ok()?,
    port => port.as_str()?.trim().parse().ok()?,
  };
  let name = [field("title"), compose.get("name").map(text)?]
    .into_iter()
    .find(|name| !name.is_empty())?;

  Some(App {
    name,
    icon: field("icon"),
    protocol: if field("scheme") == "https" {
      "https"
    } else {
      "http"
    },
    port: (port > 0).then_some(port)?,
    description: field("tagline").chars().take(120).collect(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn le_o_bloco_do_casaos() {
    let yaml = "name: jellyfin\nservices: {}\nx-casaos:\n  title: { custom: '', en_us: Jellyfin }\n  tagline: { en_us: Media }\n  icon: https://x/i.png\n  port_map: \"8097\"\n";
    let app = parse(yaml).unwrap();
    assert_eq!((app.name.as_str(), app.port), ("Jellyfin", 8097));
    assert_eq!((app.protocol, app.description.as_str()), ("http", "Media"));
  }

  #[test]
  fn ignora_app_sem_porta_ou_sem_bloco() {
    assert_eq!(parse("name: a\nx-casaos: { port_map: '' }"), None);
    assert_eq!(parse("name: a\nservices: {}"), None);
    assert_eq!(parse(": : quebrado ["), None);
  }
}
