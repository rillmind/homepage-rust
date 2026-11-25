use super::{client::Client, compose::Project};
use crate::{
  casaos,
  error::{AppError, blocking},
  files::Root,
  services::{ServiceInput, probe},
};
use serde::Serialize;
use serde_json::Value;
use std::{
  collections::HashSet,
  env, fs,
  path::Path,
  sync::{Arc, Mutex},
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::time::sleep;

/// Tempo em que o contêiner precisa continuar de pé para contar como iniciado.
const SETTLE: Duration = Duration::from_secs(4);
/// Quanto esperar a porta responder.
const PORT_WAIT: Duration = Duration::from_secs(40);

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
  Start,
  Restart,
  Update,
}

#[derive(Serialize)]
pub struct ContainerView {
  name: String,
  running: bool,
  /// Criado pelo Docker Compose, então pode ser atualizado.
  compose: bool,
}

#[derive(Serialize)]
pub struct Outcome {
  message: String,
  /// Subiu, mas algo não está certo.
  warning: bool,
  details: String,
}

#[derive(Clone)]
pub struct Docker {
  client: Client,
  root: Root,
  busy: Arc<Mutex<HashSet<String>>>,
}

/// Libera o contêiner mesmo se a ação falhar.
struct Busy(Arc<Mutex<HashSet<String>>>, String);

impl Drop for Busy {
  fn drop(&mut self) {
    self.0.lock().unwrap().remove(&self.1);
  }
}

fn text<'a>(value: &'a Value, pointer: &str) -> &'a str {
  value.pointer(pointer).and_then(Value::as_str).unwrap_or("")
}

pub fn label<'a>(container: &'a Value, name: &str) -> &'a str {
  let key = format!("com.docker.compose.{name}");
  container["Labels"][key].as_str().unwrap_or("")
}

fn name(container: &Value) -> String {
  let name = text(container, "/Names/0").trim_start_matches('/');
  match name {
    "" => text(container, "/Name").trim_start_matches('/').to_string(),
    name => name.to_string(),
  }
}

fn is_running(container: &Value) -> bool {
  matches!(
    text(container, "/State"),
    "running" | "restarting" | "paused"
  )
}

/// Portas em uso agora. Contêiner parado não tem nenhuma.
fn publishes(container: &Value, port: u16) -> bool {
  let ports = container["Ports"].as_array().into_iter().flatten();
  ports
    .filter(|item| item["Type"] == "tcp")
    .any(|item| item["PublicPort"] == port)
}

/// Portas configuradas, que valem também para o contêiner parado.
fn binds(details: &Value, port: u16) -> bool {
  let bindings = details["HostConfig"]["PortBindings"].as_object();
  bindings
    .into_iter()
    .flatten()
    .filter(|(inside, _)| inside.ends_with("/tcp"))
    .flat_map(|(_, hosts)| hosts.as_array().into_iter().flatten())
    .any(|host| text(host, "/HostPort").parse() == Ok(port))
}

/// O contêiner e o que ele precisa, na ordem em que devem subir: dependências primeiro.
fn start_order(all: &[Value], main: &Value) -> Vec<Value> {
  fn visit(all: &[Value], item: &Value, seen: &mut HashSet<String>, order: &mut Vec<Value>) {
    if !seen.insert(text(item, "/Id").to_string()) {
      return;
    }
    // "banco:service_started:false,cache:service_healthy:true"
    for dependency in label(item, "depends_on").split(',') {
      let service = dependency.split(':').next().unwrap_or_default();
      let found = all.iter().find(|other| {
        !service.is_empty()
          && label(other, "service") == service
          && label(other, "project") == label(item, "project")
      });
      if let Some(found) = found {
        visit(all, found, seen, order);
      }
    }
    order.push(item.clone());
  }
  let mut order = Vec::new();
  visit(all, main, &mut HashSet::new(), &mut order);
  order
}

fn now() -> i64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_or(0, |time| time.as_secs() as i64)
}

/// O próprio contêiner aparece nos caminhos montados pelo Docker dentro dele.
fn is_self(id: &str) -> bool {
  !id.is_empty() && fs::read_to_string("/proc/self/mountinfo").is_ok_and(|info| info.contains(id))
}

impl Docker {
  pub fn new(root: Root) -> Self {
    let socket = match env::var("DOCKER_SOCKET") {
      Ok(path) if !path.is_empty() => path.into(),
      _ => root.real(Path::new("/run/docker.sock")),
    };
    Self {
      client: Client { socket },
      root,
      busy: Arc::default(),
    }
  }

  async fn inspect(&self, id: &str) -> Result<Value, AppError> {
    self.client.get(&format!("/containers/{id}/json")).await
  }

  /// Nome informado, depois a porta, depois os apps do CasaOS.
  async fn find(&self, service: &ServiceInput) -> Result<(Vec<Value>, Value), AppError> {
    let all = self.client.get("/containers/json?all=1").await?;
    let all = all.as_array().cloned().unwrap_or_default();
    let missing = |message: String| AppError::Docker(message, String::new());

    if !service.container.is_empty() {
      let found = all.iter().find(|item| name(item) == service.container);
      let found = found
        .cloned()
        .ok_or_else(|| missing(format!("O contêiner \"{}\" não existe", service.container)))?;
      return Ok((all, found));
    }

    let port = service.port;
    if let Some(found) = all.iter().find(|item| publishes(item, port)).cloned() {
      return Ok((all, found));
    }
    for item in all.iter().filter(|item| !is_running(item)) {
      // Pode ter sido removido entre a lista e a consulta.
      let details = self.inspect(text(item, "/Id")).await;
      if details.is_ok_and(|details| binds(&details, port)) {
        let found = item.clone();
        return Ok((all, found));
      }
    }
    let root = self.root.clone();
    if let Some((project, main)) = blocking(move || Ok(casaos::find_by_port(&root, port))).await? {
      let found = all
        .iter()
        .find(|item| label(item, "project") == project && label(item, "service") == main);
      if let Some(found) = found.cloned() {
        return Ok((all, found));
      }
    }
    Err(missing(format!(
      "Nenhum contêiner usa a porta {port}. Informe o nome dele em Editar"
    )))
  }

  pub async fn view(&self, service: ServiceInput) -> Result<ContainerView, AppError> {
    let (_, container) = self.find(&service).await?;
    Ok(ContainerView {
      name: name(&container),
      running: is_running(&container),
      compose: Project::of(&container).is_some(),
    })
  }

  /// Em uma tarefa própria: fechar a página no meio não interrompe a ação.
  pub async fn run(&self, service: ServiceInput, action: Action) -> Result<Outcome, AppError> {
    let docker = self.clone();
    let task = tokio::spawn(async move { docker.execute(service, action).await });
    task
      .await
      .unwrap_or_else(|err| Err(AppError::Io(std::io::Error::other(err))))
  }

  async fn execute(&self, service: ServiceInput, action: Action) -> Result<Outcome, AppError> {
    let (all, main) = self.find(&service).await?;
    let id = text(&main, "/Id").to_string();
    if action == Action::Update {
      return self.update(&main, service.port).await;
    }
    if is_self(&id) {
      return Err(AppError::Conflict(
        "Este é o contêiner da própria homepage, reinicie pelo CasaOS ou pelo terminal",
      ));
    }
    let _busy = self.reserve(&main)?;
    let since = now();

    if action == Action::Restart {
      let result = self.client.post(&format!("/containers/{id}/restart")).await;
      self.with_logs(result, &id, &name(&main), since).await?;
    } else {
      for item in start_order(&all, &main)
        .iter()
        .filter(|item| !is_running(item))
      {
        let item_id = text(item, "/Id");
        let result = self
          .client
          .post(&format!("/containers/{item_id}/start"))
          .await;
        self.with_logs(result, item_id, &name(item), since).await?;
      }
    }
    self
      .verify(&id, &name(&main), service.port, since, action)
      .await
  }

  /// Uma ação por vez em cada projeto do compose (ou contêiner avulso).
  fn reserve(&self, container: &Value) -> Result<Busy, AppError> {
    let key = match label(container, "project") {
      "" => text(container, "/Id").to_string(),
      project => project.to_string(),
    };
    if !self.busy.lock().unwrap().insert(key.clone()) {
      return Err(AppError::Conflict(
        "Já há uma ação em andamento neste serviço",
      ));
    }
    Ok(Busy(self.busy.clone(), key))
  }

  /// docker compose pull e up -d no projeto do serviço.
  async fn update(&self, main: &Value, port: u16) -> Result<Outcome, AppError> {
    let fail = |message: &str| AppError::Docker(message.into(), String::new());
    let project =
      Project::of(main).ok_or_else(|| fail("Este contêiner não foi criado pelo Docker Compose"))?;
    project.check()?;
    let _busy = self.reserve(main)?;
    let since = now();

    project.pull(&self.client).await?;
    if is_self(text(main, "/Id")) {
      // Este processo vai ser trocado: deixa o up rodando sozinho e responde antes.
      project.up_later(&self.client).await?;
      return Ok(Outcome {
        message: "Imagem baixada. A homepage reinicia em instantes, se houver versão nova".into(),
        warning: false,
        details: String::new(),
      });
    }
    project.up(&self.client).await?;

    // O contêiner pode ter sido recriado, com outro id.
    let all = self.client.get("/containers/json?all=1").await?;
    let updated = all
      .as_array()
      .into_iter()
      .flatten()
      .find(|item| {
        label(item, "project") == label(main, "project")
          && label(item, "service") == label(main, "service")
      })
      .ok_or_else(|| fail("O contêiner sumiu depois da atualização"))?;
    let mut outcome = self
      .verify(
        text(updated, "/Id"),
        &name(updated),
        port,
        since,
        Action::Update,
      )
      .await?;
    if text(updated, "/ImageID") == text(main, "/ImageID") && !outcome.warning {
      outcome.message = format!("{} já estava na versão mais recente", name(updated));
    }
    Ok(outcome)
  }

  /// Completa o erro com o nome do contêiner e o log dele.
  async fn with_logs(
    &self,
    result: Result<(), AppError>,
    id: &str,
    name: &str,
    since: i64,
  ) -> Result<(), AppError> {
    match result {
      Err(AppError::Docker(message, _)) => {
        let logs = self.client.logs(id, since).await;
        Err(AppError::Docker(format!("{name}: {message}"), logs))
      }
      result => result,
    }
  }

  /// O Docker aceitar o comando não quer dizer que o app subiu: confere se o contêiner
  /// continua de pé e se a porta responde.
  async fn verify(
    &self,
    id: &str,
    name: &str,
    port: u16,
    since: i64,
    action: Action,
  ) -> Result<Outcome, AppError> {
    let start = Instant::now();
    let mut restarts = None;
    loop {
      let details = self.inspect(id).await?;
      let count = details["RestartCount"].as_u64().unwrap_or(0);
      let looping = restarts.is_some_and(|first| count > first);
      restarts.get_or_insert(count);

      let state = &details["State"];
      if state["Running"] != true || state["Restarting"] == true || looping {
        let code = state["ExitCode"].as_i64().unwrap_or(0);
        let reason = match (text(state, "/Error"), state["OOMKilled"] == true) {
          (_, true) => "faltou memória".to_string(),
          ("", _) if code == 0 => "encerrou sozinho".to_string(),
          ("", _) => format!("encerrou com o código {code}"),
          (error, _) => error.to_string(),
        };
        let logs = self.client.logs(id, since).await;
        let message = format!("{name} iniciou, mas parou em seguida: {reason}");
        return Err(AppError::Docker(message, logs));
      }

      let online = probe(port).await.online;
      if online && start.elapsed() >= SETTLE {
        let verb = match action {
          Action::Start => "iniciado",
          Action::Restart => "reiniciado",
          Action::Update => "atualizado",
        };
        return Ok(Outcome {
          message: format!("{name} {verb}"),
          warning: false,
          details: String::new(),
        });
      }
      if start.elapsed() >= PORT_WAIT {
        return Ok(Outcome {
          message: format!("{name} está rodando, mas a porta {port} ainda não responde"),
          warning: true,
          details: self.client.logs(id, since).await,
        });
      }
      sleep(Duration::from_secs(1)).await;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn acha_pela_porta_ligado_ou_parado() {
    let running = json!({ "Ports": [{ "PublicPort": 8097, "Type": "tcp" }, { "PublicPort": 53, "Type": "udp" }] });
    assert!(publishes(&running, 8097));
    assert!(!publishes(&running, 53));
    let stopped = json!({ "HostConfig": { "PortBindings": { "8096/tcp": [{ "HostPort": "8097" }], "53/udp": [{ "HostPort": "53" }] } } });
    assert!(binds(&stopped, 8097));
    assert!(!binds(&stopped, 8096));
    assert!(!binds(&stopped, 53));
    assert!(!binds(
      &json!({ "HostConfig": { "PortBindings": null } }),
      1
    ));
  }

  #[test]
  fn inicia_as_dependencias_antes() {
    let container = |service: &str, depends: &str| {
      json!({ "Id": service, "Labels": {
        "com.docker.compose.project": "immich",
        "com.docker.compose.service": service,
        "com.docker.compose.depends_on": depends,
      } })
    };
    let all = [
      container(
        "server",
        "redis:service_started:false,db:service_healthy:true",
      ),
      container("redis", ""),
      container("db", "redis:service_started:false"),
      container("outro", ""),
    ];
    let order = start_order(&all, &all[0]);
    let ids: Vec<&str> = order.iter().map(|item| text(item, "/Id")).collect();
    assert_eq!(ids, ["redis", "db", "server"]);
  }

  #[test]
  fn dependencia_circular_nao_trava() {
    let all = [json!({ "Id": "a", "Labels": {
      "com.docker.compose.project": "p",
      "com.docker.compose.service": "a",
      "com.docker.compose.depends_on": "a:service_started:false",
    } })];
    assert_eq!(start_order(&all, &all[0]).len(), 1);
  }
}
