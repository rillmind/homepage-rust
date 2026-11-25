use serde::Serialize;
use std::{
  net::{IpAddr, Ipv4Addr},
  time::{Duration, Instant},
};
use tokio::{net::TcpStream, time::timeout};

const TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Serialize)]
pub struct Status {
  pub online: bool,
  pub ms: Option<u64>,
}

/// Verifica se algo escuta na porta. Tenta o loopback e depois os outros IPs da máquina,
/// para o caso de o serviço escutar só em um deles, como o da VPN.
pub async fn probe(port: u16) -> Status {
  let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);
  let others = if_addrs::get_if_addrs().unwrap_or_default();
  let ips = others
    .into_iter()
    .filter(|addr| !addr.is_loopback() && !addr.is_link_local())
    .map(|addr| addr.ip());

  for ip in std::iter::once(loopback).chain(ips) {
    let start = Instant::now();
    if let Ok(Ok(_)) = timeout(TIMEOUT, TcpStream::connect((ip, port))).await {
      return Status {
        online: true,
        ms: Some(start.elapsed().as_millis() as u64),
      };
    }
  }
  Status {
    online: false,
    ms: None,
  }
}
