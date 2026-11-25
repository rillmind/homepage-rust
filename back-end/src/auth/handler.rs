use super::store::{Auth, SESSION_SECONDS};
use crate::error::{AppError, blocking};
use axum::{
  Json, Router,
  extract::{Request, State},
  http::{HeaderMap, Method, header},
  middleware::Next,
  response::{IntoResponse, Response},
  routing::{get, post},
};
use serde::{Deserialize, Serialize};

const COOKIE: &str = "homepage_session";
/// Cookies não são separados por porta: com um caminho só nosso, o navegador não manda a
/// sessão para os outros serviços do mesmo host.
const COOKIE_PATH: &str = "/homepage-api";

#[derive(Serialize)]
struct Status {
  username: Option<String>,
  authenticated: bool,
}

#[derive(Deserialize)]
struct Credentials {
  username: String,
  password: String,
}

#[derive(Deserialize)]
struct PasswordChange {
  current: String,
  new: String,
}

pub fn routes(auth: Auth) -> Router {
  Router::new()
    .route("/auth/status", get(status))
    .route("/auth/register", post(register))
    .route("/auth/login", post(login))
    .route("/auth/logout", post(logout))
    .route("/auth/password", post(password))
    .with_state(auth)
}

fn token(headers: &HeaderMap) -> Option<&str> {
  headers
    .get_all(header::COOKIE)
    .iter()
    .filter_map(|value| value.to_str().ok())
    .flat_map(|value| value.split(';'))
    .find_map(|pair| pair.trim().strip_prefix(COOKIE)?.strip_prefix('='))
}

fn cookie(token: &str, max_age: u64) -> Response {
  let value =
    format!("{COOKIE}={token}; Path={COOKIE_PATH}; HttpOnly; SameSite=Strict; Max-Age={max_age}");
  ([(header::SET_COOKIE, value)], Json(())).into_response()
}

fn authenticated(auth: &Auth, headers: &HeaderMap) -> bool {
  token(headers).is_some_and(|token| auth.is_valid(token))
}

pub async fn require(
  State(auth): State<Auth>,
  request: Request,
  next: Next,
) -> Result<Response, AppError> {
  if !authenticated(&auth, request.headers()) {
    return Err(AppError::Unauthorized("Entre para continuar"));
  }
  Ok(next.run(request).await)
}

/// Outro site não consegue mandar um cabeçalho próprio sem um preflight, que é recusado.
/// O SameSite não basta, porque os serviços em outras portas contam como o mesmo site.
pub async fn same_origin(request: Request, next: Next) -> Result<Response, AppError> {
  let safe = matches!(*request.method(), Method::GET | Method::HEAD);
  if !safe && !request.headers().contains_key("x-homepage") {
    return Err(AppError::Forbidden("Requisição de outra origem"));
  }
  Ok(next.run(request).await)
}

async fn status(State(auth): State<Auth>, headers: HeaderMap) -> Json<Status> {
  Json(Status {
    username: auth.username(),
    authenticated: authenticated(&auth, &headers),
  })
}

async fn register(
  State(auth): State<Auth>,
  Json(input): Json<Credentials>,
) -> Result<Response, AppError> {
  let token = blocking(move || auth.register(&input.username, &input.password)).await?;
  Ok(cookie(&token, SESSION_SECONDS))
}

async fn login(
  State(auth): State<Auth>,
  Json(input): Json<Credentials>,
) -> Result<Response, AppError> {
  let token = blocking(move || auth.login(&input.username, &input.password)).await?;
  Ok(cookie(&token, SESSION_SECONDS))
}

async fn logout(State(auth): State<Auth>, headers: HeaderMap) -> Result<Response, AppError> {
  if let Some(token) = token(&headers).filter(|token| auth.is_valid(token)) {
    auth.logout(token)?;
  }
  Ok(cookie("", 0))
}

async fn password(
  State(auth): State<Auth>,
  headers: HeaderMap,
  Json(input): Json<PasswordChange>,
) -> Result<Response, AppError> {
  if !authenticated(&auth, &headers) {
    return Err(AppError::Unauthorized("Entre para continuar"));
  }
  let token = blocking(move || auth.change_password(&input.current, &input.new)).await?;
  Ok(cookie(&token, SESSION_SECONDS))
}

#[cfg(test)]
mod tests {
  use super::*;
  use axum::{body::Body, middleware};
  use serde_json::{Value, json};
  use std::{env, fs, path::PathBuf};
  use tower::ServiceExt;

  struct Temp(PathBuf);

  impl Temp {
    fn new(test: &str) -> Self {
      Self(env::temp_dir().join(format!("auth-{}-{test}", std::process::id())))
    }
    fn app(&self) -> Router {
      let auth = Auth::open(self.0.join("auth.json")).unwrap();
      Router::new()
        .route("/private", get(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(auth.clone(), require))
        .merge(routes(auth))
        .layer(middleware::from_fn(same_origin))
    }
  }

  impl Drop for Temp {
    fn drop(&mut self) {
      let _ = fs::remove_dir_all(&self.0);
    }
  }

  async fn call(app: &Router, uri: &str, cookie: &str, body: Option<Value>) -> (u16, String) {
    let request = Request::builder()
      .method(if body.is_some() { "POST" } else { "GET" })
      .uri(uri)
      .header("content-type", "application/json")
      .header("cookie", cookie)
      .header("x-homepage", "1")
      .body(Body::from(body.map(|b| b.to_string()).unwrap_or_default()))
      .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let cookie = response
      .headers()
      .get("set-cookie")
      .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string())
      .unwrap_or_default();
    (response.status().as_u16(), cookie)
  }

  fn user(password: &str) -> Option<Value> {
    Some(json!({ "username": "raul", "password": password }))
  }

  #[tokio::test]
  async fn cadastro_unico_login_e_logout() {
    let temp = Temp::new("flow");
    let app = temp.app();
    assert_eq!(call(&app, "/private", "", None).await.0, 401);
    assert_eq!(call(&app, "/auth/register", "", user("12345")).await.0, 422);

    let (status, first) = call(&app, "/auth/register", "", user("senha-forte")).await;
    assert_eq!(status, 200);
    assert_eq!(call(&app, "/private", &first, None).await.0, 200);
    assert_eq!(
      call(&app, "/auth/register", "", user("outra-senha"))
        .await
        .0,
      409
    );

    // A sessão sobrevive ao reinício e o arquivo não guarda senha nem token.
    let app = temp.app();
    assert_eq!(call(&app, "/private", &first, None).await.0, 200);
    let saved = fs::read_to_string(temp.0.join("auth.json")).unwrap();
    assert!(!saved.contains("senha-forte"));
    assert!(!saved.contains(first.split('=').nth(1).unwrap()));

    call(&app, "/auth/logout", &first, Some(json!({}))).await;
    assert_eq!(call(&app, "/private", &first, None).await.0, 401);
    assert_eq!(
      call(&app, "/auth/login", "", user("senha-forte")).await.0,
      200
    );
  }

  #[tokio::test]
  async fn bloqueia_depois_de_muitas_falhas() {
    let temp = Temp::new("lock");
    let app = temp.app();
    call(&app, "/auth/register", "", user("senha-forte")).await;
    for _ in 0..5 {
      assert_eq!(
        call(&app, "/auth/login", "", user("errada-123")).await.0,
        401
      );
    }
    assert_eq!(
      call(&app, "/auth/login", "", user("senha-forte")).await.0,
      429
    );
  }

  #[tokio::test]
  async fn trocar_senha_encerra_outras_sessoes() {
    let temp = Temp::new("password");
    let app = temp.app();
    let (_, old) = call(&app, "/auth/register", "", user("senha-forte")).await;
    let body = Some(json!({ "current": "senha-forte", "new": "123456" }));
    assert_eq!(call(&app, "/auth/password", "", body.clone()).await.0, 401);
    let (_, new) = call(&app, "/auth/password", &old, body).await;
    assert_eq!(call(&app, "/private", &old, None).await.0, 401);
    assert_eq!(call(&app, "/private", &new, None).await.0, 200);
  }

  #[tokio::test]
  async fn recusa_escrita_de_outra_origem() {
    let app = Temp::new("csrf").app();
    let request = Request::post("/auth/logout").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status().as_u16(), 403);
  }
}
