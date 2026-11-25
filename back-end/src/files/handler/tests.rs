use super::*;
use axum::body::to_bytes;
use std::{env, fs, os::unix::fs::symlink, time::Duration};

/// Uma máquina falsa em uma pasta temporária, apagada ao final.
struct Machine {
  dir: PathBuf,
  app: Router,
}

impl Machine {
  fn new(test: &str) -> Self {
    let dir = env::temp_dir().join(format!("files-{}-{test}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    for path in [
      "root/home/raul/docs",
      "root/home/raul/fotos",
      "root/DATA/AppData/x",
      "root/proc",
    ] {
      fs::create_dir_all(dir.join(path)).unwrap();
    }
    fs::write(dir.join("root/home/raul/docs/a.txt"), "olá").unwrap();
    fs::write(dir.join("segredo"), "fora").unwrap();
    let root = Root::new(dir.join("root"), "/DATA/AppData");
    let app = routes(root, &dir.join("data")).unwrap();
    Self { dir, app }
  }

  fn path(&self, path: &str) -> PathBuf {
    self.dir.join("root").join(path)
  }

  async fn raw(&self, method: &str, uri: &str, body: Body, range: &str) -> Response {
    let mut request = Request::builder()
      .method(method)
      .uri(uri)
      .header("content-type", "application/json");
    if !range.is_empty() {
      request = request.header("range", range);
    }
    let request = request.body(body).unwrap();
    self.app.clone().oneshot(request).await.unwrap()
  }

  async fn call(&self, method: &str, uri: &str, body: Value) -> (u16, Value) {
    let response = self
      .raw(method, uri, Body::from(body.to_string()), "")
      .await;
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
  }

  async fn get(&self, uri: &str) -> (u16, Value) {
    self.call("GET", uri, Value::Null).await
  }

  async fn transfer(&self, mode: &str, items: &[&str], to: &str) -> u16 {
    let body = json!({ "mode": mode, "items": items, "to": to });
    self.call("POST", "/files/transfer", body).await.0
  }

  async fn wait(&self) -> Value {
    for _ in 0..200 {
      let (_, jobs) = self.get("/files/jobs").await;
      let last = jobs.as_array().unwrap().last().unwrap().clone();
      if last["state"] != "waiting" && last["state"] != "running" {
        return last;
      }
      tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("o trabalho não terminou");
  }
}

impl Drop for Machine {
  fn drop(&mut self) {
    let _ = fs::remove_dir_all(&self.dir);
  }
}

#[tokio::test]
async fn links_e_pontos_nao_saem_da_raiz() {
  let machine = Machine::new("escape");
  symlink(machine.dir.join("segredo"), machine.path("home/raul/abs")).unwrap();
  symlink("../../../../../segredo", machine.path("home/raul/rel")).unwrap();
  symlink("/home/raul/docs", machine.path("home/raul/atalho")).unwrap();
  symlink("volta", machine.path("home/raul/volta")).unwrap();

  for path in ["/home/raul/abs", "/home/raul/rel", "/../segredo"] {
    let (status, _) = machine.get(&format!("/files/download?path={path}")).await;
    assert_eq!(status, 404, "{path}");
  }
  assert_eq!(
    machine.get("/files/list?path=/home/raul/volta").await.0,
    422
  );
  assert_eq!(machine.get("/files/list?path=home").await.0, 422);
  // Link absoluto vale a partir da raiz da máquina.
  let (_, listing) = machine.get("/files/list?path=/home/raul/atalho").await;
  assert_eq!(listing["path"], "/home/raul/docs");
}

#[tokio::test]
async fn cria_renomeia_e_nao_sobrescreve() {
  let machine = Machine::new("rename");
  let new = |name: &str| json!({ "dir": "/home/raul", "name": name });
  assert_eq!(
    machine.call("POST", "/files/folder", new("nova")).await.0,
    201
  );
  assert_eq!(
    machine.call("POST", "/files/folder", new("nova")).await.0,
    409
  );
  for name in ["..", "a/b", "", ".homepage-trash"] {
    assert_eq!(
      machine.call("POST", "/files/file", new(name)).await.0,
      422,
      "{name}"
    );
  }
  let rename = |name: &str| json!({ "path": "/home/raul/nova", "name": name });
  assert_eq!(
    machine
      .call("POST", "/files/rename", rename("docs"))
      .await
      .0,
    409
  );
  assert_eq!(
    machine.call("POST", "/files/rename", rename("ok")).await.0,
    204
  );
  assert!(machine.path("home/raul/ok").is_dir());
}

#[tokio::test]
async fn protege_pastas_do_sistema_e_appdata() {
  let machine = Machine::new("protect");
  for path in [
    "/home",
    "/DATA/AppData",
    "/DATA/AppData/x",
    "/DATA",
    "/proc",
    "/",
  ] {
    let (status, _) = machine
      .call("POST", "/files/trash", json!({ "items": [path] }))
      .await;
    assert_eq!(status, 403, "{path}");
  }
  let new = json!({ "dir": "/DATA/AppData/x", "name": "y" });
  assert_eq!(machine.call("POST", "/files/folder", new).await.0, 403);
  // Nem passando por um link.
  symlink("/DATA/AppData", machine.path("home/raul/app")).unwrap();
  let new = json!({ "dir": "/home/raul/app", "name": "y" });
  assert_eq!(machine.call("POST", "/files/folder", new).await.0, 403);
}

#[tokio::test]
async fn copia_e_move() {
  let machine = Machine::new("transfer");
  assert_eq!(
    machine
      .transfer("copy", &["/home/raul/docs"], "/home/raul/fotos")
      .await,
    200
  );
  assert_eq!(machine.wait().await["state"], "done");
  assert_eq!(
    fs::read_to_string(machine.path("home/raul/fotos/docs/a.txt")).unwrap(),
    "olá"
  );

  // Copiar no mesmo lugar numera em vez de sobrescrever.
  assert_eq!(
    machine
      .transfer("copy", &["/home/raul/docs/a.txt"], "/home/raul/docs")
      .await,
    200
  );
  machine.wait().await;
  assert!(machine.path("home/raul/docs/a (1).txt").exists());

  assert_eq!(
    machine
      .transfer("move", &["/home/raul/docs/a.txt"], "/home/raul")
      .await,
    200
  );
  assert_eq!(machine.wait().await["state"], "done");
  assert!(!machine.path("home/raul/docs/a.txt").exists());
  assert!(machine.path("home/raul/a.txt").exists());

  assert_eq!(
    machine
      .transfer("copy", &["/home/raul"], "/home/raul/docs")
      .await,
    422
  );
}

#[tokio::test]
async fn copia_que_falha_nao_deixa_sobras() {
  let machine = Machine::new("partial");
  let status = std::process::Command::new("mkfifo")
    .arg(machine.path("home/raul/docs/fila"))
    .status()
    .unwrap();
  assert!(status.success());
  machine
    .transfer("copy", &["/home/raul/docs"], "/home/raul/fotos")
    .await;
  assert_eq!(machine.wait().await["state"], "failed");
  assert_eq!(
    fs::read_dir(machine.path("home/raul/fotos"))
      .unwrap()
      .count(),
    0
  );
  assert!(machine.path("home/raul/docs/a.txt").exists());
}

#[tokio::test]
async fn lixeira_restaura_e_apaga() {
  let machine = Machine::new("trash");
  let file = json!({ "items": ["/home/raul/docs/a.txt"] });
  assert_eq!(
    machine.call("POST", "/files/trash", file.clone()).await.0,
    204
  );
  assert!(!machine.path("home/raul/docs/a.txt").exists());

  let (_, items) = machine.get("/files/trash").await;
  let id = items[0]["id"].as_str().unwrap().to_string();
  // O nome foi ocupado enquanto isso.
  fs::write(machine.path("home/raul/docs/a.txt"), "outro").unwrap();
  let uri = format!("/files/trash/{id}/restore");
  assert_eq!(machine.call("POST", &uri, Value::Null).await.0, 204);
  assert_eq!(
    fs::read_to_string(machine.path("home/raul/docs/a (1).txt")).unwrap(),
    "olá"
  );

  machine.call("POST", "/files/trash", file).await;
  let (_, items) = machine.get("/files/trash").await;
  let location = items[0]["location"].as_str().unwrap().to_string();
  let ids = json!({ "items": [items[0]["id"]] });
  machine.call("DELETE", "/files/trash", ids).await;
  assert_eq!(machine.wait().await["state"], "done");
  assert!(!machine.path(&location[1..]).exists());
  assert_eq!(machine.get("/files/trash").await.1, json!([]));
}

#[tokio::test]
async fn envio_e_download_com_range() {
  let machine = Machine::new("upload");
  let uri = "/files/upload?dir=/home/raul/docs&name=b.mp4";
  let big = vec![7u8; 10 * 1024 * 1024];
  let response = machine.raw("POST", uri, Body::from(big), "").await;
  assert_eq!(response.status().as_u16(), 201);
  assert_eq!(
    fs::metadata(machine.path("home/raul/docs/b.mp4"))
      .unwrap()
      .len(),
    10 << 20
  );

  let uri = "/files/download?inline=true&path=/home/raul/docs/b.mp4";
  let response = machine.raw("GET", uri, Body::empty(), "bytes=0-9").await;
  assert_eq!(response.status().as_u16(), 206);
  assert!(
    response.headers()["content-disposition"]
      .to_str()
      .unwrap()
      .starts_with("inline")
  );
  assert_eq!(to_bytes(response.into_body(), 100).await.unwrap().len(), 10);
}

#[tokio::test]
async fn html_e_svg_sempre_baixam() {
  let machine = Machine::new("inline");
  for name in ["x.html", "x.svg", "x.js"] {
    fs::write(machine.path("home/raul").join(name), "<script>").unwrap();
    let uri = format!("/files/download?inline=true&path=/home/raul/{name}");
    let response = machine.raw("GET", &uri, Body::empty(), "").await;
    let value = response.headers()["content-disposition"].to_str().unwrap();
    assert!(value.starts_with("attachment"), "{name}");
  }
}

#[tokio::test]
async fn texto_detecta_mudanca_por_fora() {
  let machine = Machine::new("text");
  let (_, text) = machine.get("/files/text?path=/home/raul/docs/a.txt").await;
  assert_eq!(text["content"], "olá");
  let save = json!({ "path": "/home/raul/docs/a.txt", "content": "x", "version": text["version"] });
  assert_eq!(
    machine.call("PUT", "/files/text", save.clone()).await.0,
    200
  );
  assert_eq!(
    fs::read_to_string(machine.path("home/raul/docs/a.txt")).unwrap(),
    "x"
  );
  assert_eq!(machine.call("PUT", "/files/text", save).await.0, 409);

  fs::write(machine.path("home/raul/bin"), [0, 1, 2]).unwrap();
  assert_eq!(machine.get("/files/text?path=/home/raul/bin").await.0, 422);
}

#[tokio::test]
async fn busca_e_miniatura() {
  let machine = Machine::new("search");
  let (_, result) = machine.get("/files/search?path=/&q=A.TX").await;
  assert_eq!(result["found"][0]["path"], "/home/raul/docs/a.txt");

  let image = image::RgbImage::new(600, 300);
  image.save(machine.path("home/raul/f.png")).unwrap();
  let response = machine
    .raw(
      "GET",
      "/files/thumb?path=/home/raul/f.png",
      Body::empty(),
      "",
    )
    .await;
  assert_eq!(response.status().as_u16(), 200);
  let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
  assert_eq!(image::load_from_memory(&bytes).unwrap().width(), 240);
  assert_eq!(
    machine
      .get("/files/thumb?path=/home/raul/docs/a.txt")
      .await
      .0,
    422
  );
}
