use super::{client::Client, container::label};
use crate::error::AppError;
use serde_json::{Value, json};
use std::{
  collections::BTreeSet,
  env,
  path::{Path, PathBuf},
  time::{Duration, Instant},
};
use tokio::time::sleep;

const IMAGE: &str = "docker.io/library/docker";
const TAG: &str = "cli";
const LIMIT: Duration = Duration::from_secs(20 * 60);

/// Recusa variável sem valor antes de mexer em algo: o app seria recriado com ela vazia.
const PULL: &str = r#"
check=$(docker compose "$@" config -q 2>&1) || { echo "$check"; exit 1; }
case "$check" in *"variable is not set"*) echo "$check"; exit 42;; esac
docker compose "$@" pull --ignore-buildable
"#;
const UP: &str = r#"docker compose "$@" up -d"#;
/// Dá tempo de responder ao navegador.
const UP_LATER: &str = r#"sleep 3; docker compose "$@" up -d"#;

/// Projeto do Docker Compose, lido dos rótulos que ele põe no contêiner.
#[derive(Debug, PartialEq)]
pub struct Project {
  name: String,
  dir: String,
  files: Vec<String>,
  env_files: Vec<String>,
}

impl Project {
  pub fn of(container: &Value) -> Option<Self> {
    let list = |name| -> Vec<String> {
      label(container, name)
        .split(',')
        .filter(|path| !path.is_empty())
        .map(String::from)
        .collect()
    };
    let project = Self {
      name: label(container, "project").into(),
      dir: label(container, "project.working_dir").into(),
      files: list("project.config_files"),
      env_files: list("project.environment_file"),
    };
    let complete = !project.name.is_empty() && !project.files.is_empty();
    complete.then_some(project)
  }

  /// Pastas montadas no ajudante, no mesmo caminho da máquina.
  fn dirs(&self) -> BTreeSet<PathBuf> {
    let parents = self.files.iter().chain(&self.env_files);
    parents
      .filter_map(|path| Path::new(path).parent())
      .chain(Some(Path::new(&self.dir)).filter(|dir| *dir != Path::new("")))
      .map(Path::to_path_buf)
      .collect()
  }

  pub fn check(&self) -> Result<(), AppError> {
    let valid = |dir: &PathBuf| {
      dir.is_absolute() && dir.components().count() > 1 && !dir.to_string_lossy().contains(':')
    };
    if !self.dirs().iter().all(valid) {
      return Err(AppError::Docker(
        "Não foi possível localizar o compose deste serviço".into(),
        String::new(),
      ));
    }
    Ok(())
  }

  fn args(&self) -> Vec<String> {
    let mut args = vec!["--project-name".into(), self.name.clone()];
    if !self.dir.is_empty() {
      args.extend(["--project-directory".into(), self.dir.clone()]);
    }
    for file in &self.files {
      args.extend(["--file".into(), file.clone()]);
    }
    for file in &self.env_files {
      args.extend(["--env-file".into(), file.clone()]);
    }
    args
  }

  fn config(&self, script: &str, auto_remove: bool) -> Value {
    let socket = env::var("DOCKER_SOCKET_HOST")
      .ok()
      .filter(|path| !path.is_empty())
      .unwrap_or_else(|| "/var/run/docker.sock".into());
    let mut binds = vec![format!("{socket}:/var/run/docker.sock")];
    for dir in self.dirs() {
      binds.push(format!("{0}:{0}:ro", dir.display()));
    }
    json!({
      "Image": format!("{IMAGE}:{TAG}"),
      // Os caminhos vão como argumentos, nunca dentro do script.
      "Entrypoint": ["sh", "-c", script, "sh"],
      "Cmd": self.args(),
      "Env": ["DOCKER_HOST=unix:///var/run/docker.sock", "COMPOSE_PROGRESS=plain", "COMPOSE_MENU=0"],
      "Labels": { "homepage.helper": "update" },
      "HostConfig": {
        "Binds": binds,
        "AutoRemove": auto_remove,
        "SecurityOpt": ["label=disable"],
      },
    })
  }

  pub async fn pull(&self, client: &Client) -> Result<(), AppError> {
    client.ensure_image(IMAGE, TAG).await?;
    match self.run(client, PULL).await? {
      (0, _) => Ok(()),
      (42, log) => Err(AppError::Docker(
        "Faltam variáveis do compose, então nada foi alterado. Atualize pelo CasaOS".into(),
        log,
      )),
      (_, log) => Err(AppError::Docker(
        "Falha ao baixar a atualização, nada foi alterado".into(),
        log,
      )),
    }
  }

  pub async fn up(&self, client: &Client) -> Result<(), AppError> {
    match self.run(client, UP).await? {
      (0, _) => Ok(()),
      (_, log) => Err(AppError::Docker("Falha ao recriar o serviço".into(), log)),
    }
  }

  pub async fn up_later(&self, client: &Client) -> Result<(), AppError> {
    let id = client.create(&self.config(UP_LATER, true)).await?;
    client.post(&format!("/containers/{id}/start")).await
  }

  /// Roda o ajudante até o fim e devolve o código de saída e o log.
  async fn run(&self, client: &Client, script: &str) -> Result<(i64, String), AppError> {
    let id = client.create(&self.config(script, false)).await?;
    let result = self.wait(client, &id).await;
    let log = client.logs(&id, 0).await;
    client.remove(&id).await;
    Ok((result?, log))
  }

  async fn wait(&self, client: &Client, id: &str) -> Result<i64, AppError> {
    client.post(&format!("/containers/{id}/start")).await?;
    let start = Instant::now();
    while start.elapsed() < LIMIT {
      let details = client.get(&format!("/containers/{id}/json")).await?;
      if details["State"]["Running"] != true {
        return Ok(details["State"]["ExitCode"].as_i64().unwrap_or(1));
      }
      sleep(Duration::from_secs(1)).await;
    }
    Err(AppError::Docker(
      "A atualização passou de 20 minutos e foi interrompida".into(),
      String::new(),
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn container(dir: &str, files: &str) -> Value {
    json!({ "Labels": {
      "com.docker.compose.project": "jellyfin",
      "com.docker.compose.project.working_dir": dir,
      "com.docker.compose.project.config_files": files,
    } })
  }

  #[test]
  fn le_o_projeto_dos_rotulos() {
    let project = Project::of(&container("/apps/jf", "/apps/jf/a.yml,/outro/b.yml")).unwrap();
    assert!(project.check().is_ok());
    let dirs: Vec<PathBuf> = project.dirs().into_iter().collect();
    assert_eq!(dirs, [PathBuf::from("/apps/jf"), PathBuf::from("/outro")]);
    let config = project.config(UP, false);
    assert_eq!(config["HostConfig"]["Binds"][1], "/apps/jf:/apps/jf:ro");
    assert!(
      config["Cmd"]
        .as_array()
        .unwrap()
        .contains(&json!("/outro/b.yml"))
    );
  }

  #[test]
  fn recusa_conteiner_avulso_e_pastas_estranhas() {
    assert_eq!(Project::of(&json!({ "Labels": {} })), None);
    for (dir, file) in [
      ("/", "/a.yml"),
      ("", "relativo.yml"),
      ("/a:b", "/a:b/c.yml"),
    ] {
      assert!(Project::of(&container(dir, file)).unwrap().check().is_err());
    }
  }
}
