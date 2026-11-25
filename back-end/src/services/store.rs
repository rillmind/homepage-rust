use super::models::{Service, ServiceInput};
use crate::{error::AppError, storage};
use std::{
  io,
  path::PathBuf,
  sync::{Arc, Mutex, MutexGuard},
};

/// Serviços em memória, gravados em um arquivo JSON a cada mudança.
#[derive(Clone)]
pub struct Store {
  path: Arc<PathBuf>,
  services: Arc<Mutex<Vec<Service>>>,
}

impl Store {
  pub fn open(path: PathBuf) -> io::Result<Self> {
    let services = storage::load(&path)?;
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

  pub fn get(&self, id: u64) -> Result<Service, AppError> {
    let services = self.lock();
    let service = services.iter().find(|s| s.id == id);
    service.cloned().ok_or(AppError::NotFound)
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
    storage::save(&self.path, &next)?;
    *current = next;
    Ok(())
  }
}
