use super::{
  jobs::{JobView, Jobs},
  models::*,
  ops,
  root::{Root, check_name},
  thumbs, transfer,
  trash::{Trash, Trashed},
};
use crate::error::{AppError, blocking};
use axum::{
  Json, Router,
  body::Body,
  extract::{DefaultBodyLimit, Path as UrlPath, Query, Request, State},
  http::{HeaderValue, StatusCode, header},
  response::{IntoResponse, Response},
  routing::{delete, get, post},
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::{
  io,
  path::{Path, PathBuf},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};
use tokio::{io::AsyncWriteExt, sync::Semaphore};
use tower::ServiceExt;
use tower_http::services::ServeFile;

#[derive(Clone)]
struct Files {
  root: Root,
  jobs: Jobs,
  trash: Trash,
  thumbs: Arc<PathBuf>,
  thumb_turn: Arc<Semaphore>,
  search_turn: Arc<Semaphore>,
}

pub fn routes(root: Root, data: &Path) -> io::Result<Router> {
  let thumb_dir = data.join("thumbs");
  thumbs::trim(&thumb_dir);
  let files = Files {
    root,
    jobs: Jobs::default(),
    trash: Trash::open(data.join("trash.json"))?,
    thumbs: Arc::new(thumb_dir),
    thumb_turn: Arc::new(Semaphore::new(1)),
    search_turn: Arc::new(Semaphore::new(1)),
  };
  Ok(
    Router::new()
      .route("/files/list", get(list))
      .route("/files/search", get(search))
      .route("/files/download", get(download))
      .route("/files/thumb", get(thumb))
      .route("/files/text", get(read_text).put(save_text))
      .route("/files/folder", post(create_folder))
      .route("/files/file", post(create_file))
      .route("/files/rename", post(rename))
      .route("/files/transfer", post(start_transfer))
      .route("/files/jobs", get(list_jobs))
      .route("/files/jobs/{id}", delete(cancel_job))
      .route(
        "/files/trash",
        get(list_trash).post(put_trash).delete(purge),
      )
      .route("/files/trash/{id}/restore", post(restore))
      .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
      // O envio grava direto no disco, sem limite de tamanho.
      .route("/files/upload", post(upload))
      .with_state(files),
  )
}

async fn list(
  State(files): State<Files>,
  Query(query): Query<PathQuery>,
) -> Result<Json<Listing>, AppError> {
  blocking(move || ops::list(&files.root, &query.path))
    .await
    .map(Json)
}

/// Avisa a busca para parar quando o navegador desiste dela.
struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
  fn drop(&mut self) {
    self.0.store(true, Ordering::Relaxed);
  }
}

async fn search(
  State(files): State<Files>,
  Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResult>, AppError> {
  let cancelled = Arc::new(AtomicBool::new(false));
  let _guard = CancelOnDrop(cancelled.clone());
  let turn = files.search_turn.clone().acquire_owned().await;
  blocking(move || {
    let _turn = turn;
    ops::search(&files.root, &query.path, &query.q, &cancelled)
  })
  .await
  .map(Json)
}

/// Tipos que o navegador mostra sem executar nada. O resto vira download, porque um HTML
/// aberto aqui rodaria com a sessão do usuário.
fn safe_inline(mime: &str) -> bool {
  let mime = mime.split(';').next().unwrap_or_default().trim();
  matches!(
    mime,
    "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "image/avif" | "image/bmp"
  ) || mime == "application/pdf"
    || mime.starts_with("video/")
    || mime.starts_with("audio/")
}

fn percent_encode(text: &str) -> String {
  text
    .bytes()
    .map(|byte| match byte {
      b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (byte as char).to_string(),
      _ => format!("%{byte:02X}"),
    })
    .collect()
}

async fn serve(path: &Path, request: Request) -> Result<Response, AppError> {
  let response = ServeFile::new(path)
    .oneshot(request)
    .await
    .map_err(|err| AppError::Io(io::Error::other(err)))?;
  let mut response = response.map(Body::new);
  response.headers_mut().insert(
    header::X_CONTENT_TYPE_OPTIONS,
    HeaderValue::from_static("nosniff"),
  );
  Ok(response)
}

/// Aceita Range, então dá para avançar um vídeo sem baixar tudo.
async fn download(
  State(files): State<Files>,
  Query(query): Query<DownloadQuery>,
  request: Request,
) -> Result<Response, AppError> {
  let root = files.root.clone();
  let (path, real) = blocking(move || {
    let path = root.resolve(&query.path)?;
    let real = root.real(&path);
    ops::open_regular(&real)?;
    Ok((path, real))
  })
  .await?;

  let mut response = serve(&real, request).await?;
  let headers = response.headers_mut();
  let mime = headers
    .get(header::CONTENT_TYPE)
    .and_then(|value| value.to_str().ok())
    .unwrap_or_default();
  let kind = if query.inline && safe_inline(mime) {
    "inline"
  } else {
    "attachment"
  };
  let name = path.file_name().unwrap_or_default().to_string_lossy();
  let value = format!("{kind}; filename*=UTF-8''{}", percent_encode(&name));
  if let Ok(value) = HeaderValue::from_str(&value) {
    headers.insert(header::CONTENT_DISPOSITION, value);
  }
  headers.insert(
    header::CACHE_CONTROL,
    HeaderValue::from_static("private, no-cache"),
  );
  Ok(response)
}

async fn thumb(
  State(files): State<Files>,
  Query(query): Query<PathQuery>,
  request: Request,
) -> Result<Response, AppError> {
  // Uma por vez, para não ocupar a CPU toda.
  let turn = files.thumb_turn.clone().acquire_owned().await;
  let path = blocking(move || {
    let _turn = turn;
    thumbs::get(&files.root, &files.thumbs, &query.path)
  })
  .await?;
  let mut response = serve(&path, request).await?;
  response.headers_mut().insert(
    header::CACHE_CONTROL,
    HeaderValue::from_static("private, max-age=3600"),
  );
  Ok(response)
}

async fn read_text(
  State(files): State<Files>,
  Query(query): Query<PathQuery>,
) -> Result<Json<Text>, AppError> {
  blocking(move || ops::read_text(&files.root, &query.path))
    .await
    .map(Json)
}

async fn save_text(
  State(files): State<Files>,
  Json(input): Json<SaveText>,
) -> Result<Json<Text>, AppError> {
  blocking(move || ops::save_text(&files.root, &input))
    .await
    .map(Json)
}

async fn create_folder(
  State(files): State<Files>,
  Json(input): Json<NewItem>,
) -> Result<StatusCode, AppError> {
  blocking(move || ops::create(&files.root, &input.dir, &input.name, true)).await?;
  Ok(StatusCode::CREATED)
}

async fn create_file(
  State(files): State<Files>,
  Json(input): Json<NewItem>,
) -> Result<StatusCode, AppError> {
  blocking(move || ops::create(&files.root, &input.dir, &input.name, false)).await?;
  Ok(StatusCode::CREATED)
}

async fn rename(
  State(files): State<Files>,
  Json(input): Json<Rename>,
) -> Result<StatusCode, AppError> {
  blocking(move || ops::rename(&files.root, &input.path, &input.name)).await?;
  Ok(StatusCode::NO_CONTENT)
}

/// Grava em um arquivo temporário e só mostra quando chega inteiro.
async fn upload(
  State(files): State<Files>,
  Query(input): Query<NewItem>,
  body: Body,
) -> Result<Response, AppError> {
  let root = files.root.clone();
  let (dir, name) = blocking(move || {
    check_name(&input.name)?;
    let dir = root.resolve(&input.dir)?;
    root.check_write(&dir)?;
    Ok((root.real(&dir), input.name))
  })
  .await?;

  let partial = transfer::Partial::new(&dir);
  let mut file = tokio::fs::File::create_new(&partial.0).await?;
  let mut stream = body.into_data_stream();
  while let Some(chunk) = stream.next().await {
    let chunk = chunk.map_err(|_| AppError::Invalid("O envio foi interrompido"))?;
    file.write_all(&chunk).await?;
  }
  file.flush().await?;
  drop(file);

  let placed = blocking(move || {
    ops::inherit_owner(&partial.0);
    Ok(ops::place(&partial.0, &dir, &name)?)
  })
  .await?;
  let name = placed.file_name().unwrap_or_default().to_string_lossy();
  Ok((StatusCode::CREATED, Json(json!({ "name": name }))).into_response())
}

async fn start_transfer(
  State(files): State<Files>,
  Json(input): Json<Transfer>,
) -> Result<Json<Value>, AppError> {
  let root = files.root.clone();
  let (items, dir, mode) = blocking(move || {
    let (items, dir) = transfer::prepare(&root, &input)?;
    Ok((items, dir, input.mode))
  })
  .await?;
  let verb = if mode == Mode::Copy {
    "Copiar"
  } else {
    "Mover"
  };
  let label = format!("{verb} {} para {}", plural(items.len()), dir.display());
  let root = files.root.clone();
  let id = files.jobs.start(label, move |progress| {
    transfer::run(&root, &items, &dir, mode, progress)
  });
  Ok(Json(json!({ "id": id })))
}

fn plural(count: usize) -> String {
  match count {
    1 => "1 item".into(),
    _ => format!("{count} itens"),
  }
}

async fn list_jobs(State(files): State<Files>) -> Json<Vec<JobView>> {
  Json(files.jobs.list())
}

async fn cancel_job(
  State(files): State<Files>,
  UrlPath(id): UrlPath<u64>,
) -> Result<StatusCode, AppError> {
  files.jobs.cancel(id)?;
  Ok(StatusCode::NO_CONTENT)
}

async fn list_trash(State(files): State<Files>) -> Json<Vec<Trashed>> {
  Json(files.trash.list())
}

async fn put_trash(
  State(files): State<Files>,
  Json(input): Json<Items>,
) -> Result<StatusCode, AppError> {
  blocking(move || {
    input
      .items
      .iter()
      .try_for_each(|item| files.trash.put(&files.root, item))
  })
  .await?;
  Ok(StatusCode::NO_CONTENT)
}

async fn restore(
  State(files): State<Files>,
  UrlPath(id): UrlPath<String>,
) -> Result<StatusCode, AppError> {
  blocking(move || files.trash.restore(&files.root, &id)).await?;
  Ok(StatusCode::NO_CONTENT)
}

async fn purge(State(files): State<Files>, Json(input): Json<Items>) -> Json<Value> {
  let label = format!("Excluir {} da lixeira", plural(input.items.len()));
  let (root, trash) = (files.root.clone(), files.trash.clone());
  let id = files.jobs.start(label, move |progress| {
    trash.purge(&root, &input.items, progress)
  });
  Json(json!({ "id": id }))
}

#[cfg(test)]
mod tests;
