use super::models::{Cpu, Disk, Memory};
use std::fs;

pub fn hostname() -> Option<String> {
  let name = fs::read_to_string("/proc/sys/kernel/hostname").ok()?;
  Some(name.trim().to_string())
}

pub fn memory() -> Option<Memory> {
  parse_memory(&fs::read_to_string("/proc/meminfo").ok()?)
}

fn parse_memory(text: &str) -> Option<Memory> {
  let bytes = |key: &str| -> Option<u64> {
    let value = text.lines().find_map(|line| line.strip_prefix(key))?;
    Some(value.split_whitespace().next()?.parse::<u64>().ok()? * 1024)
  };
  let total = bytes("MemTotal:")?;
  let available = bytes("MemAvailable:")?;

  Some(Memory {
    total,
    used: total.saturating_sub(available),
  })
}

#[derive(Debug, PartialEq)]
pub struct CpuTimes {
  total: u64,
  idle: u64,
  cores: usize,
}

pub fn cpu_times() -> Option<CpuTimes> {
  parse_cpu_times(&fs::read_to_string("/proc/stat").ok()?)
}

fn parse_cpu_times(text: &str) -> Option<CpuTimes> {
  let mut lines = text.lines();
  // user nice system idle iowait irq softirq steal. Os campos guest ficam de fora
  // porque o kernel já os soma em user e nice.
  let times = lines
    .next()?
    .strip_prefix("cpu ")?
    .split_whitespace()
    .take(8)
    .map(|n| n.parse().ok())
    .collect::<Option<Vec<u64>>>()?;

  Some(CpuTimes {
    total: times.iter().sum(),
    idle: times.get(3)? + times.get(4)?,
    cores: lines.take_while(|line| line.starts_with("cpu")).count(),
  })
}

fn busy_percent(before: &CpuTimes, after: &CpuTimes) -> f64 {
  let total = after.total.saturating_sub(before.total);
  let idle = after.idle.saturating_sub(before.idle);
  if total == 0 {
    return 0.0;
  }
  total.saturating_sub(idle) as f64 / total as f64 * 100.0
}

/// Frequência máxima em MHz ou, se o kernel não informar (VMs), a atual.
fn cpu_mhz() -> Option<f64> {
  if let Ok(text) = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
    && let Ok(khz) = text.trim().parse::<f64>()
  {
    return Some(khz / 1000.0);
  }
  fs::read_to_string("/proc/cpuinfo")
    .ok()?
    .lines()
    .find(|line| line.starts_with("cpu MHz"))?
    .split_once(':')?
    .1
    .trim()
    .parse()
    .ok()
}

pub fn cpu(before: &CpuTimes, after: &CpuTimes) -> Cpu {
  Cpu {
    mhz: cpu_mhz(),
    percent: busy_percent(before, after),
    cores: after.cores,
  }
}

pub fn disk(path: &str) -> Option<Disk> {
  let stat = rustix::fs::statvfs(path).ok()?;
  Some(disk_usage(
    stat.f_blocks * stat.f_frsize,
    stat.f_bfree * stat.f_frsize,
    stat.f_bavail * stat.f_frsize,
  ))
}

fn disk_usage(total: u64, free: u64, available: u64) -> Disk {
  let used = total.saturating_sub(free);
  // Mesma conta do df: o espaço reservado ao root não entra na porcentagem.
  let usable = used + available;

  Disk {
    total,
    used,
    free: available,
    percent: if usable == 0 {
      0.0
    } else {
      used as f64 / usable as f64 * 100.0
    },
  }
}

pub fn uptime() -> Option<u64> {
  parse_uptime(&fs::read_to_string("/proc/uptime").ok()?)
}

fn parse_uptime(text: &str) -> Option<u64> {
  let seconds: f64 = text.split_whitespace().next()?.parse().ok()?;
  Some(seconds as u64)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn memoria_usada_e_total_menos_disponivel() {
    let text = "MemTotal: 16777216 kB\nMemFree: 1 kB\nMemAvailable: 4194304 kB\n";
    let expected = Memory {
      total: 16 << 30,
      used: 12 << 30,
    };
    assert_eq!(parse_memory(text), Some(expected));
    assert_eq!(parse_memory("MemTotal: 1 kB\n"), None);
  }

  #[test]
  fn cpu_conta_nucleos_e_ignora_guest() {
    let text = "cpu  10 0 10 70 10 0 0 0 99 99\ncpu0 1 1\ncpu1 1 1\nintr 5\n";
    let expected = CpuTimes {
      total: 100,
      idle: 80,
      cores: 2,
    };
    assert_eq!(parse_cpu_times(text), Some(expected));
  }

  #[test]
  fn cpu_percentual_entre_duas_leituras() {
    let before = CpuTimes {
      total: 100,
      idle: 80,
      cores: 2,
    };
    let after = CpuTimes {
      total: 200,
      idle: 155,
      cores: 2,
    };
    assert_eq!(busy_percent(&before, &after), 25.0);
    assert_eq!(busy_percent(&before, &before), 0.0);
  }

  #[test]
  fn disco_desconta_espaco_reservado() {
    const G: u64 = 1 << 30;
    let disk = disk_usage(100 * G, 40 * G, 20 * G);
    assert_eq!(
      (disk.total, disk.used, disk.free),
      (100 * G, 60 * G, 20 * G)
    );
    assert_eq!(disk.percent, 75.0);
  }

  #[test]
  fn uptime_descarta_fracao() {
    assert_eq!(parse_uptime("12345.67 999.0\n"), Some(12345));
    assert_eq!(parse_uptime(""), None);
  }
}
