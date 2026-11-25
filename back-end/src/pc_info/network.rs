use super::models::Interface;
use std::{
  collections::HashMap,
  fs,
  net::{IpAddr, UdpSocket},
};

/// Bytes recebidos e enviados por interface.
type Counters = HashMap<String, (u64, u64)>;

pub fn counters() -> Counters {
  fs::read_to_string("/proc/net/dev")
    .map(|text| parse_counters(&text))
    .unwrap_or_default()
}

fn parse_counters(text: &str) -> Counters {
  text
    .lines()
    .filter_map(|line| {
      let (name, values) = line.split_once(':')?;
      let values: Vec<&str> = values.split_whitespace().collect();
      let rx = values.first()?.parse().ok()?;
      let tx = values.get(8)?.parse().ok()?;
      Some((name.trim().to_string(), (rx, tx)))
    })
    .collect()
}

/// IP da interface que sai para a internet. O connect de UDP só consulta a tabela de
/// rotas, nenhum pacote é enviado.
pub fn primary_ip() -> Option<IpAddr> {
  let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
  socket.connect("1.1.1.1:53").ok()?;
  Some(socket.local_addr().ok()?.ip())
}

fn rate(before: u64, after: u64, seconds: f64) -> f64 {
  if seconds <= 0.0 {
    return 0.0;
  }
  after.saturating_sub(before) as f64 / seconds
}

pub fn interfaces(before: &Counters, after: &Counters, seconds: f64) -> Vec<Interface> {
  let addrs = if_addrs::get_if_addrs().unwrap_or_default();
  let primary = primary_ip();

  let mut list: Vec<Interface> = after
    .iter()
    .filter(|(name, _)| *name != "lo" && !name.starts_with("veth"))
    .map(|(name, &(rx, tx))| {
      let &(rx_before, tx_before) = before.get(name).unwrap_or(&(rx, tx));
      let file = |file| fs::read_to_string(format!("/sys/class/net/{name}/{file}")).ok();
      let ips: Vec<IpAddr> = addrs
        .iter()
        .filter(|addr| addr.name == *name)
        .map(|addr| addr.ip())
        .collect();
      Interface {
        up: file("carrier").is_some_and(|carrier| carrier.trim() == "1"),
        primary: primary.is_some_and(|ip| ips.contains(&ip)),
        ips,
        rx: rate(rx_before, rx, seconds),
        tx: rate(tx_before, tx, seconds),
        // O kernel devolve -1 quando o link não informa velocidade.
        speed: file("speed").and_then(|speed| speed.trim().parse().ok()),
        name: name.clone(),
      }
    })
    .collect();
  list.sort_by(|a, b| {
    (b.primary, b.up)
      .cmp(&(a.primary, a.up))
      .then_with(|| a.name.cmp(&b.name))
  });
  list
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn le_contadores_do_proc() {
    let text = "Inter-| Receive | Transmit\n face |bytes packets|bytes\n\
                eth0: 1000 1 0 0 0 0 0 0 500 1 0 0 0 0 0 0\n";
    let counters = parse_counters(text);
    assert_eq!(counters.len(), 1);
    assert_eq!(counters["eth0"], (1000, 500));
  }

  #[test]
  fn taxa_nao_fica_negativa_se_o_contador_zerar() {
    assert_eq!(rate(1000, 3000, 2.0), 1000.0);
    assert_eq!(rate(3000, 0, 1.0), 0.0);
  }
}
