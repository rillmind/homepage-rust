use super::{
  jobs::Progress,
  ops,
  root::{Root, TRASH_DIR},
  transfer,
};
use crate::{auth::random_id, error::AppError, storage};
use serde::{Deserialize, Serialize};
use std::{
  fs::{self, DirBuilder},
  io,
  os::unix::fs::{DirBuilderExt, MetadataExt},
  path::{Path, PathBuf},
  sync::{Arc, Mutex},
  time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trashed {
  pub id: String,
  pub original: String,
  /// Onde o item está agora.
  pub location: String,
  pub deleted: u64,
  pub is_dir: bool,
}

/// Cada disco tem a sua lixeira, então excluir e restaurar é só renomear.
#[derive(Clone)]
pub struct Trash {
  path: Arc<PathBuf>,
  items: Arc<Mutex<Vec<Trashed>>>,
}

impl Trash {
  pub fn open(path: PathBuf) -> io::Result<Self> {
    Ok(Self {
      items: Arc::new(Mutex::new(storage::load(&path)?)),
      path: Arc::new(path),
    })
  }

  pub fn list(&self) -> Vec<Trashed> {
    self.items.lock().unwrap().clone()
  }

  /// A pasta mais alta do mesmo disco em que dá para criar a lixeira.
  fn dir(root: &Root, item: &Path) -> Result<PathBuf, AppError> {
    let device = fs::symlink_metadata(root.real(item))?.dev();
    let parents: Vec<&Path> = item
      .ancestors()
      .skip(1)
      .take_while(|dir| fs::metadata(root.real(dir)).is_ok_and(|meta| meta.dev() == device))
      .collect();

    for parent in parents.into_iter().rev() {
      let dir = parent.join(TRASH_DIR);
      let real = root.real(&dir);
      let created = match DirBuilder::new().mode(0o700).create(&real) {
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
          // Uma lixeira de outro usuário, ou um link no lugar dela, não serve.
          fs::symlink_metadata(&real)
            .is_ok_and(|meta| meta.is_dir() && meta.uid() == rustix::process::geteuid().as_raw())
        }
        result => result.is_ok(),
      };
      if created {
        return Ok(dir);
      }
    }
    Err(AppError::Forbidden("Sem permissão para excluir aqui"))
  }

  pub fn put(&self, root: &Root, path: &str) -> Result<(), AppError> {
    let item = root.resolve_link(path)?;
    root.check_move(&item)?;
    let real = root.real(&item);
    transfer::check_mounts(&real)?;
    let is_dir = fs::symlink_metadata(&real)?.is_dir();

    let id = random_id(8);
    let location = Self::dir(root, &item)?.join(&id);
    let mut items = self.items.lock().unwrap();
    let mut next = items.clone();
    next.push(Trashed {
      original: item.to_string_lossy().into_owned(),
      location: location.to_string_lossy().into_owned(),
      deleted: SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs()),
      is_dir,
      id,
    });
    // Registra antes de mover: uma queda no meio não deixa o item perdido na lixeira.
    storage::save(&self.path, &next)?;
    if let Err(err) = ops::rename_new(&real, &root.real(&location)) {
      storage::save(&self.path, &*items)?;
      return Err(err.into());
    }
    *items = next;
    Ok(())
  }

  fn find(&self, id: &str) -> Result<Trashed, AppError> {
    let items = self.items.lock().unwrap();
    let item = items.iter().find(|item| item.id == id);
    item.cloned().ok_or(AppError::NotFound)
  }

  fn forget(&self, id: &str) -> Result<(), AppError> {
    let mut items = self.items.lock().unwrap();
    let mut next = items.clone();
    next.retain(|item| item.id != id);
    storage::save(&self.path, &next)?;
    *items = next;
    Ok(())
  }

  /// Volta para a pasta de origem, com um número no nome se ele já estiver em uso.
  pub fn restore(&self, root: &Root, id: &str) -> Result<(), AppError> {
    let item = self.find(id)?;
    let original = Path::new(&item.original);
    let (Some(parent), Some(name)) = (original.parent(), original.file_name()) else {
      return Err(AppError::NotFound);
    };
    let dir = root
      .resolve(&parent.to_string_lossy())
      .map_err(|_| AppError::Conflict("A pasta de origem não existe mais"))?;
    root.check_write(&dir)?;
    let from = root.real(Path::new(&item.location));
    ops::place(&from, &root.real(&dir), &name.to_string_lossy())?;
    self.forget(id)
  }

  pub fn purge(&self, root: &Root, ids: &[String], progress: &Progress) -> Result<(), AppError> {
    for id in ids {
      progress.check()?;
      let item = self.find(id)?;
      let real = root.real(Path::new(&item.location));
      transfer::check_mounts(&real)?;
      match transfer::remove(&real) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => return Err(err.into()),
        _ => self.forget(id)?,
      }
    }
    Ok(())
  }
}
