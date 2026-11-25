use super::{
  models::{Cpu, Interface, Stats},
  network, sensors, system,
};
use axum::{Json, Router, extract::State, routing::get};
use chrono::Local;
use std::{
  env,
  path::Path,
  sync::{Arc, RwLock},
  time::{Duration, Instant},
};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// O que depende de duas leituras no tempo, medido em segundo plano.
#[derive(Default)]
struct Sample {
  cpu: Option<Cpu>,
  network: Vec<Interface>,
}

type Shared = Arc<RwLock<Sample>>;

pub fn routes() -> Router {
  let shared = Shared::default();
  tokio::spawn(sampler(shared.clone()));
  Router::new().route("/stats", get(stats)).with_state(shared)
}

async fn sampler(shared: Shared) {
  let mut cpu_before = system::cpu_times();
  let mut net_before = network::counters();
  let mut read_at = Instant::now();

  loop {
    tokio::time::sleep(SAMPLE_INTERVAL).await;
    let cpu_after = system::cpu_times();
    let net_after = network::counters();
    let elapsed = read_at.elapsed().as_secs_f64();
    read_at = Instant::now();

    let sample = Sample {
      cpu: cpu_before
        .as_ref()
        .zip(cpu_after.as_ref())
        .map(|(before, after)| system::cpu(before, after)),
      network: network::interfaces(&net_before, &net_after, elapsed),
    };
    *shared.write().unwrap() = sample;
    cpu_before = cpu_after;
    net_before = net_after;
  }
}

async fn stats(State(shared): State<Shared>) -> Json<Stats> {
  let now = Local::now();
  let fan_path = env::var_os("CPU_FAN_PATH").filter(|path| !path.is_empty());
  // Em container o `/` não é o do host, por isso o caminho é configurável.
  let disk_path = env::var("DISK_PATH").unwrap_or_else(|_| "/".into());
  let (cpu, network) = {
    let sample = shared.read().unwrap();
    (sample.cpu.clone(), sample.network.clone())
  };

  Json(Stats {
    hostname: system::hostname(),
    ip: network::primary_ip(),
    date: now.format("%d/%m/%Y").to_string(),
    time: now.format("%H:%M").to_string(),
    uptime: system::uptime(),
    cpu,
    memory: system::memory(),
    disk: system::disk(&disk_path),
    temp: sensors::read(
      Path::new("/sys/class/hwmon"),
      fan_path.as_deref().map(Path::new),
    ),
    network,
  })
}
