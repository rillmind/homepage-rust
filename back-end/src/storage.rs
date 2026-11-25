use serde::{Serialize, de::DeserializeOwned};
use std::{
  fs::{self, OpenOptions},
  io::{self, Write},
  os::unix::fs::OpenOptionsExt,
  path::Path,
};

/// Arquivo inválido é erro, para não sobrescrever os dados com um valor vazio.
pub fn load<T: DeserializeOwned + Default>(path: &Path) -> io::Result<T> {
  match fs::read(path) {
    Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
    Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(T::default()),
    Err(err) => Err(err),
  }
}

/// Grava ao lado e renomeia, para uma queda no meio não corromper o arquivo.
pub fn save<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
  if let Some(dir) = path.parent().filter(|dir| *dir != Path::new("")) {
    fs::create_dir_all(dir)?;
  }
  let temp = path.with_extension("json.tmp");
  let mut file = OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .mode(0o600)
    .open(&temp)?;
  file.write_all(&serde_json::to_vec_pretty(value)?)?;
  file.sync_all()?;
  fs::rename(&temp, path)
}
