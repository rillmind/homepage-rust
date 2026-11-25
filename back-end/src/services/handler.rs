use super::{
  models::{Service, ServiceInput},
  probe::{Status, probe},
  store::Store,
};
use crate::{
  docker::{Action, ContainerView, Docker, Outcome},
  error::AppError,
};
use axum::{
  Json, Router,
  extract::{Path, State},
  http::StatusCode,
  routing::{get, post, put},
};
use std::collections::BTreeMap;
use tokio::task::JoinSet;

pub fn routes(store: Store, docker: Docker) -> Router {
  let containers = Router::new()
    .route("/services/{id}/container", get(container))
    .route("/services/{id}/start", post(start))
    .route("/services/{id}/restart", post(restart))
    .with_state((store.clone(), docker));

  Router::new()
    .route("/services", get(list).post(create))
    .route("/services/status", get(status))
    .route("/services/{id}", put(update).delete(delete))
    .route("/probe/{port}", get(probe_port))
    .with_state(store)
    .merge(containers)
}

async fn list(State(store): State<Store>) -> Json<Vec<Service>> {
  Json(store.list())
}

async fn create(
  State(store): State<Store>,
  Json(input): Json<ServiceInput>,
) -> Result<(StatusCode, Json<Service>), AppError> {
  let service = store.create(input.validate()?)?;
  Ok((StatusCode::CREATED, Json(service)))
}

async fn update(
  State(store): State<Store>,
  Path(id): Path<u64>,
  Json(input): Json<ServiceInput>,
) -> Result<Json<Service>, AppError> {
  Ok(Json(store.update(id, input.validate()?)?))
}

async fn delete(State(store): State<Store>, Path(id): Path<u64>) -> Result<StatusCode, AppError> {
  store.delete(id)?;
  Ok(StatusCode::NO_CONTENT)
}

/// Status de todos os serviços, por id, verificados ao mesmo tempo.
async fn status(State(store): State<Store>) -> Json<BTreeMap<u64, Status>> {
  let mut probes: JoinSet<_> = store
    .list()
    .into_iter()
    .map(|service| async move { (service.id, probe(service.input.port).await) })
    .collect();
  let mut result = BTreeMap::new();
  while let Some(probe) = probes.join_next().await {
    result.extend(probe.ok());
  }
  Json(result)
}

async fn probe_port(Path(port): Path<u16>) -> Json<Status> {
  Json(probe(port).await)
}

type Containers = State<(Store, Docker)>;

async fn container(
  State((store, docker)): Containers,
  Path(id): Path<u64>,
) -> Result<Json<ContainerView>, AppError> {
  docker.view(store.get(id)?.input).await.map(Json)
}

async fn start(
  State((store, docker)): Containers,
  Path(id): Path<u64>,
) -> Result<Json<Outcome>, AppError> {
  docker
    .run(store.get(id)?.input, Action::Start)
    .await
    .map(Json)
}

async fn restart(
  State((store, docker)): Containers,
  Path(id): Path<u64>,
) -> Result<Json<Outcome>, AppError> {
  docker
    .run(store.get(id)?.input, Action::Restart)
    .await
    .map(Json)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::files::Root;
  use axum::{
    body::{Body, to_bytes},
    http::Request,
  };
  use serde_json::{Value, json};
  use std::{env, fs, path::PathBuf};
  use tower::ServiceExt;

  struct Temp(PathBuf);

  impl Temp {
    fn new(test: &str) -> Self {
      Self(env::temp_dir().join(format!("services-{}-{test}", std::process::id())))
    }
    fn file(&self) -> PathBuf {
      self.0.join("services.json")
    }
  }

  impl Drop for Temp {
    fn drop(&mut self) {
      let _ = fs::remove_dir_all(&self.0);
    }
  }

  async fn call(app: &Router, method: &str, uri: &str, body: Value) -> (u16, Value) {
    let request = Request::builder()
      .method(method)
      .uri(uri)
      .header("content-type", "application/json")
      .body(Body::from(body.to_string()))
      .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
  }

  fn app(temp: &Temp) -> Router {
    let root = Root::new(temp.0.clone(), "");
    routes(Store::open(temp.file()).unwrap(), Docker::new(root))
  }

  fn jellyfin() -> Value {
    json!({ "name": " Jellyfin ", "protocol": "http", "port": 8096, "category": "Mídia" })
  }

  #[tokio::test]
  async fn crud_persiste_no_arquivo() {
    let temp = Temp::new("crud");
    let app = app(&temp);

    let (status, created) = call(&app, "POST", "/services", jellyfin()).await;
    assert_eq!((status, &created["name"]), (201, &json!("Jellyfin")));

    let mut edited = jellyfin();
    edited["port"] = json!(9000);
    assert_eq!(call(&app, "PUT", "/services/1", edited).await.0, 200);

    let reopened = Store::open(temp.file()).unwrap();
    assert_eq!(reopened.list()[0].input.port, 9000);

    assert_eq!(
      call(&app, "DELETE", "/services/1", Value::Null).await.0,
      204
    );
    assert_eq!(
      call(&app, "DELETE", "/services/1", Value::Null).await.0,
      404
    );
    assert!(Store::open(temp.file()).unwrap().list().is_empty());
  }

  #[tokio::test]
  async fn rejeita_dados_invalidos() {
    let temp = Temp::new("invalid");
    let app = app(&temp);
    for (field, value) in [
      ("name", json!("  ")),
      ("port", json!(0)),
      ("port", json!(70000)),
      ("icon", json!("javascript:alert(1)")),
      ("protocol", json!("ftp")),
      ("container", json!("../x")),
    ] {
      let mut body = jellyfin();
      body[field] = value;
      assert_eq!(
        call(&app, "POST", "/services", body).await.0,
        422,
        "{field}"
      );
    }
    assert!(!temp.file().exists());
  }

  #[tokio::test]
  async fn arquivo_corrompido_nao_e_sobrescrito() {
    let temp = Temp::new("corrupt");
    fs::create_dir_all(&temp.0).unwrap();
    fs::write(temp.file(), "{quebrado").unwrap();
    assert!(Store::open(temp.file()).is_err());
  }

  #[tokio::test]
  async fn status_diferencia_porta_aberta_e_fechada() {
    let temp = Temp::new("status");
    let app = app(&temp);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let mut body = jellyfin();
    body["port"] = json!(port);
    call(&app, "POST", "/services", body).await;
    let (_, online) = call(&app, "GET", "/services/status", Value::Null).await;
    assert_eq!(online["1"]["online"], true);

    drop(listener);
    let (_, offline) = call(&app, "GET", "/services/status", Value::Null).await;
    assert_eq!(offline["1"]["online"], false);
  }

  #[tokio::test]
  async fn sem_docker_explica_o_erro() {
    let temp = Temp::new("nodocker");
    let app = app(&temp);
    call(&app, "POST", "/services", jellyfin()).await;
    let (status, body) = call(&app, "POST", "/services/1/restart", Value::Null).await;
    assert_eq!(status, 502);
    assert!(body["error"].as_str().unwrap().contains("Docker"));
    assert_eq!(
      call(&app, "POST", "/services/9/start", Value::Null).await.0,
      404
    );
  }
}
