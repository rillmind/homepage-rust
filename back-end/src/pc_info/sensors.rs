use super::models::Temp;
use std::{
  fs,
  path::{Path, PathBuf},
  str::FromStr,
};

struct TempSensor {
  chip: String,
  label: String,
  input: PathBuf,
  celsius: f64,
}

fn read_text(path: &Path) -> Option<String> {
  Some(fs::read_to_string(path).ok()?.trim().to_lowercase())
}

fn read_number<T: FromStr>(path: &Path) -> Option<T> {
  read_text(path)?.parse().ok()
}

/// Troca o sufixo do arquivo do sensor: `fan1_input` -> `fan1_max`.
fn sibling(input: &Path, suffix: &str) -> PathBuf {
  let name = input.file_name().unwrap_or_default().to_string_lossy();
  input.with_file_name(name.replace("_input", suffix))
}

/// Lista os `<prefix>*_input` de todos os chips. O número do hwmon muda entre boots,
/// por isso a busca nunca depende dele.
fn inputs(root: &Path, prefix: &str) -> Vec<PathBuf> {
  let list = |dir: &Path| {
    fs::read_dir(dir)
      .into_iter()
      .flatten()
      .flatten()
      .map(|entry| entry.path())
  };
  let mut paths: Vec<_> = list(root)
    .flat_map(|chip| list(&chip))
    .filter(|path| {
      let name = path.file_name().unwrap_or_default().to_string_lossy();
      name.starts_with(prefix) && name.ends_with("_input")
    })
    .collect();
  paths.sort();
  paths
}

/// Sensor que falha na leitura é ignorado, sem derrubar os outros.
fn temp_sensors(root: &Path) -> Vec<TempSensor> {
  inputs(root, "temp")
    .into_iter()
    .filter_map(|input| {
      Some(TempSensor {
        chip: read_text(&input.with_file_name("name"))?,
        label: read_text(&sibling(&input, "_label")).unwrap_or_default(),
        celsius: read_number::<f64>(&input)? / 1000.0,
        input,
      })
    })
    .collect()
}

/// Intel, AMD, Intel sem sensor de package, ARM e, em último caso, ACPI.
fn cpu_temp(sensors: &[TempSensor]) -> Option<&TempSensor> {
  let exact = |chip: &str, label: &str| sensors.iter().find(|s| s.chip == chip && s.label == label);
  let chip = |prefix: &str| sensors.iter().find(|s| s.chip.starts_with(prefix));

  exact("coretemp", "package id 0")
    .or_else(|| exact("k10temp", "tctl"))
    .or_else(|| exact("k10temp", "tdie"))
    .or_else(|| {
      sensors
        .iter()
        .filter(|s| s.chip == "coretemp" && s.label.starts_with("core "))
        .max_by(|a, b| a.celsius.total_cmp(&b.celsius))
    })
    .or_else(|| chip("cpu_thermal"))
    .or_else(|| chip("acpitz"))
}

/// Caminho configurado, depois label de CPU, depois a única fan da máquina.
fn cpu_fan(root: &Path, configured: Option<&Path>) -> Option<PathBuf> {
  if let Some(path) = configured {
    return Some(path.to_path_buf());
  }
  let fans = inputs(root, "fan");
  let labeled = fans.iter().find(|input| {
    read_text(&sibling(input, "_label"))
      .is_some_and(|label| label.contains("cpu") || label.contains("processor"))
  });
  match (labeled, fans.as_slice()) {
    (Some(fan), _) | (None, [fan]) => Some(fan.clone()),
    _ => None,
  }
}

pub fn read(root: &Path, fan_path: Option<&Path>) -> Temp {
  let sensors = temp_sensors(root);
  let temp = cpu_temp(&sensors);
  let fan = cpu_fan(root, fan_path);

  Temp {
    celsius: temp.map(|s| s.celsius),
    critical: temp
      .and_then(|s| read_number::<f64>(&sibling(&s.input, "_crit")))
      .map(|millis| millis / 1000.0)
      .filter(|critical| *critical > 0.0),
    fan: fan.as_deref().and_then(read_number),
    fan_max: fan
      .as_deref()
      .and_then(|path| read_number(&sibling(path, "_max")))
      .filter(|max| *max > 0),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Cria um /sys/class/hwmon falso, apagado ao final do teste.
  struct Hwmon(PathBuf);

  impl Hwmon {
    fn new(test: &str, files: &[(&str, &str)]) -> Self {
      let root = std::env::temp_dir().join(format!("hwmon-{}-{test}", std::process::id()));
      for (path, content) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("{content}\n")).unwrap();
      }
      Self(root)
    }
  }

  impl Drop for Hwmon {
    fn drop(&mut self) {
      let _ = fs::remove_dir_all(&self.0);
    }
  }

  #[test]
  fn servidor_ubuntu_dell() {
    let hwmon = Hwmon::new(
      "dell",
      &[
        ("hwmon0/name", "acpitz"),
        ("hwmon0/temp1_input", "69500"),
        ("hwmon2/name", "dell_smm"),
        ("hwmon2/fan1_input", "4475"),
        ("hwmon2/fan1_label", "Processor Fan"),
        ("hwmon2/fan1_max", "5228"),
        ("hwmon3/name", "coretemp"),
        ("hwmon3/temp1_input", "71000"),
        ("hwmon3/temp1_label", "Package id 0"),
        ("hwmon3/temp1_crit", "105000"),
        ("hwmon3/temp2_input", "72000"),
        ("hwmon3/temp2_label", "Core 0"),
      ],
    );
    let expected = Temp {
      celsius: Some(71.0),
      critical: Some(105.0),
      fan: Some(4475),
      fan_max: Some(5228),
    };
    assert_eq!(read(&hwmon.0, None), expected);
  }

  #[test]
  fn amd_com_fans_sem_label_exige_configuracao() {
    let hwmon = Hwmon::new(
      "amd",
      &[
        ("hwmon0/name", "k10temp"),
        ("hwmon0/temp1_input", "50000"),
        ("hwmon0/temp1_label", "Tctl"),
        ("hwmon1/name", "nct6775"),
        ("hwmon1/fan1_input", "900"),
        ("hwmon1/fan2_input", "0"),
      ],
    );
    assert_eq!(read(&hwmon.0, None).celsius, Some(50.0));
    assert_eq!(read(&hwmon.0, None).fan, None);

    let configured = hwmon.0.join("hwmon1/fan2_input");
    assert_eq!(read(&hwmon.0, Some(&configured)).fan, Some(0));
  }

  #[test]
  fn intel_sem_package_usa_nucleo_mais_quente_e_fan_unica() {
    let hwmon = Hwmon::new(
      "cores",
      &[
        ("hwmon0/name", "coretemp"),
        ("hwmon0/temp2_input", "60000"),
        ("hwmon0/temp2_label", "Core 0"),
        ("hwmon0/temp3_input", "65000"),
        ("hwmon0/temp3_label", "Core 1"),
        ("hwmon0/temp4_input", "quebrado"),
        ("hwmon1/fan1_input", "1200"),
      ],
    );
    let temp = read(&hwmon.0, None);
    assert_eq!((temp.celsius, temp.fan), (Some(65.0), Some(1200)));
  }

  #[test]
  fn vm_sem_sensores_devolve_null() {
    let temp = read(Path::new("/caminho/inexistente"), None);
    assert_eq!(
      serde_json::to_string(&temp).unwrap(),
      r#"{"celsius":null,"critical":null,"fan":null,"fanMax":null}"#
    );
  }
}
