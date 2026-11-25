use super::models::{Service, ServiceInput};
use crate::error::AppError;
use std::{
  fs, io,
  path::{Path, PathBuf},
  sync::{Arc, Mutex, MutexGuard},
};

/// Serviços em memória, gravados em um arquivo JSON a cada mudança.
#[derive(Clone)]
pub struct Store {
  path: Arc<PathBuf>,
  services: Arc<Mutex<Vec<Service>>>,
}

impl Store {
  /// Arquivo inválido é erro, para não sobrescrever os dados com uma lista vazia.
  pub fn open(path: PathBuf) -> io::Result<Self> {
    let services = match fs::read(&path) {
      Ok(bytes) => serde_json::from_slice(&bytes)?,
      Err(err) if err.kind() == io::ErrorKind::NotFound => Vec::new(),
      Err(err) => return Err(err),
    };
    Ok(Self {
      path: Arc::new(path),
      services: Arc::new(Mutex::new(services)),
    })
  }

  fn lock(&self) -> MutexGuard<'_, Vec<Service>> {
    self.services.lock().unwrap()
  }

  pub fn list(&self) -> Vec<Service> {
    self.lock().clone()
  }

  pub fn create(&self, input: ServiceInput) -> Result<Service, AppError> {
    let mut services = self.lock();
    let id = services.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    let service = Service { id, input };
    let mut next = services.clone();
    next.push(service.clone());
    self.commit(&mut services, next)?;
    Ok(service)
  }

  pub fn update(&self, id: u64, input: ServiceInput) -> Result<Service, AppError> {
    let mut services = self.lock();
    let mut next = services.clone();
    let service = next
      .iter_mut()
      .find(|s| s.id == id)
      .ok_or(AppError::NotFound)?;
    service.input = input;
    let service = service.clone();
    self.commit(&mut services, next)?;
    Ok(service)
  }

  pub fn delete(&self, id: u64) -> Result<(), AppError> {
    let mut services = self.lock();
    let mut next = services.clone();
    next.retain(|s| s.id != id);
    if next.len() == services.len() {
      return Err(AppError::NotFound);
    }
    self.commit(&mut services, next)?;
    Ok(())
  }

  /// Só troca a lista em memória depois que o arquivo foi gravado.
  fn commit(&self, current: &mut Vec<Service>, next: Vec<Service>) -> io::Result<()> {
    if let Some(dir) = self.path.parent().filter(|dir| *dir != Path::new("")) {
      fs::create_dir_all(dir)?;
    }
    // Grava ao lado e renomeia, para uma queda no meio não corromper o arquivo.
    let temp = self.path.with_extension("json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(&next)?)?;
    fs::rename(&temp, &*self.path)?;
    *current = next;
    Ok(())
  }
}
