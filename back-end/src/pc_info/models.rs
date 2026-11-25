use serde::Serialize;
use std::net::IpAddr;

/// `None` vira `null` no JSON: a máquina não expõe o dado.
#[derive(Serialize)]
pub struct Stats {
  pub hostname: Option<String>,
  pub ip: Option<IpAddr>,
  pub date: String,
  pub time: String,
  pub uptime: Option<u64>,
  pub cpu: Option<Cpu>,
  pub memory: Option<Memory>,
  pub disk: Option<Disk>,
  pub temp: Temp,
  pub network: Vec<Interface>,
}

#[derive(Clone, Serialize)]
pub struct Cpu {
  pub mhz: Option<f64>,
  pub percent: f64,
  pub cores: usize,
}

/// Valores em bytes.
#[derive(Debug, PartialEq, Serialize)]
pub struct Memory {
  pub total: u64,
  pub used: u64,
}

/// Valores em bytes.
#[derive(Serialize)]
pub struct Disk {
  pub total: u64,
  pub used: u64,
  pub free: u64,
  pub percent: f64,
}

/// `fan: 0` é fan parada, diferente de `null`.
#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Temp {
  pub celsius: Option<f64>,
  pub critical: Option<f64>,
  pub fan: Option<u64>,
  pub fan_max: Option<u64>,
}

/// Taxas em bytes por segundo e velocidade do link em Mbps.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Interface {
  pub name: String,
  pub up: bool,
  pub primary: bool,
  pub ips: Vec<IpAddr>,
  pub rx: f64,
  pub tx: f64,
  pub speed: Option<u64>,
}
