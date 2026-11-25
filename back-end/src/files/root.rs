use crate::error::AppError;
use std::{
  collections::VecDeque,
  ffi::OsString,
  fs, io,
  path::{Component, Path, PathBuf},
  sync::Arc,
};

const MAX_LINKS: usize = 40;
/// Sistemas de arquivos do kernel: dá para ler, mas não mexer.
const VIRTUAL: [&str; 4] = ["/proc", "/sys", "/dev", "/run"];
pub const TRASH_DIR: &str = ".homepage-trash";

/// Caminhos da API são os da máquina ("/home/raul"), mesmo que em um container ela esteja
/// montada em outro lugar.
#[derive(Clone)]
pub struct Root {
  base: Arc<PathBuf>,
  read_only: Arc<Vec<PathBuf>>,
}

impl Root {
  pub fn new(base: PathBuf, read_only: &str) -> Self {
    let read_only = read_only
      .split(':')
      .filter(|path| path.starts_with('/') && path.len() > 1)
      .map(PathBuf::from)
      .collect();
    Self {
      base: Arc::new(base),
      read_only: Arc::new(read_only),
    }
  }

  /// Caminho real de um caminho já resolvido.
  pub fn real(&self, path: &Path) -> PathBuf {
    self.base.join(path.strip_prefix("/").unwrap_or(path))
  }

  /// Segue os links simbólicos como se a base fosse o `/`, então um link absoluto aponta
  /// para a máquina e não para o container.
  pub fn resolve(&self, path: &str) -> io::Result<PathBuf> {
    self.walk(path, true)
  }

  /// Não segue o último componente: opera no próprio link, não no destino.
  pub fn resolve_link(&self, path: &str) -> io::Result<PathBuf> {
    self.walk(path, false)
  }

  fn walk(&self, path: &str, follow_last: bool) -> io::Result<PathBuf> {
    let invalid = || io::Error::from(io::ErrorKind::InvalidInput);
    if !path.starts_with('/') || path.contains('\0') {
      return Err(invalid());
    }
    let parts = |path: &Path| -> Vec<OsString> {
      path
        .components()
        .filter_map(|part| match part {
          Component::Normal(name) => Some(name.to_os_string()),
          Component::ParentDir => Some("..".into()),
          _ => None,
        })
        .collect()
    };
    let mut queue = VecDeque::from(parts(Path::new(path)));
    let mut current = PathBuf::from("/");
    let mut links = 0;

    while let Some(name) = queue.pop_front() {
      if name == ".." {
        current.pop();
        continue;
      }
      let candidate = current.join(&name);
      let meta = fs::symlink_metadata(self.real(&candidate))?;
      if meta.is_symlink() && (follow_last || !queue.is_empty()) {
        links += 1;
        if links > MAX_LINKS {
          return Err(invalid());
        }
        let target = fs::read_link(self.real(&candidate))?;
        if target.is_absolute() {
          current = "/".into();
        }
        for part in parts(&target).into_iter().rev() {
          queue.push_front(part);
        }
      } else {
        current = candidate;
      }
    }
    Ok(current)
  }

  pub fn is_virtual(&self, path: &Path) -> bool {
    VIRTUAL.iter().any(|dir| path.starts_with(dir))
  }

  pub fn is_read_only(&self, path: &Path) -> bool {
    self.is_virtual(path) || self.read_only.iter().any(|dir| path.starts_with(dir))
  }

  /// Pode criar ou alterar algo em `path` (já resolvido)?
  pub fn check_write(&self, path: &Path) -> Result<(), AppError> {
    if self.is_read_only(path) {
      return Err(AppError::Forbidden("Esta pasta é protegida"));
    }
    Ok(())
  }

  /// Pode mover, renomear ou excluir `path`? Pastas da raiz (como /etc) e as que contêm
  /// uma pasta protegida não podem.
  pub fn check_move(&self, path: &Path) -> Result<(), AppError> {
    self.check_write(path)?;
    let top_level = path.components().count() <= 2;
    let holds_protected = self.read_only.iter().any(|dir| dir.starts_with(path));
    let in_trash = path.components().any(|part| part.as_os_str() == TRASH_DIR);
    if top_level || holds_protected || in_trash {
      return Err(AppError::Forbidden("Este item é protegido"));
    }
    Ok(())
  }
}

pub fn check_name(name: &str) -> Result<&str, AppError> {
  let bad = name.is_empty()
    || name.len() > 255
    || name == "."
    || name == ".."
    || name == TRASH_DIR
    || name.contains(['/', '\0']);
  if bad {
    return Err(AppError::Invalid("Nome inválido"));
  }
  Ok(name)
}
