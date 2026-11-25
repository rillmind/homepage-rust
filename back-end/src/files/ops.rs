use super::{
  models::{Entry, Found, Listing, SaveText, SearchResult, Text},
  root::{Root, check_name},
};
use crate::error::AppError;
use rustix::fs::{CWD, OFlags, RenameFlags};
use std::{
  fs::{self, File, Metadata, OpenOptions},
  io::{self, Read, Write},
  os::unix::fs::{MetadataExt, OpenOptionsExt, chown},
  path::{Path, PathBuf},
  sync::atomic::{AtomicBool, Ordering},
  time::{Duration, Instant},
};

const MAX_TEXT: u64 = 1024 * 1024;
const MAX_RESULTS: usize = 500;
const MAX_SEARCH: Duration = Duration::from_secs(20);

fn entry(root: &Root, path: &Path, name: String, meta: Metadata) -> Entry {
  let is_link = meta.is_symlink();
  let target = if is_link {
    root
      .resolve(&path.to_string_lossy())
      .and_then(|target| fs::metadata(root.real(&target)))
      .unwrap_or(meta)
  } else {
    meta
  };
  Entry {
    name,
    is_dir: target.is_dir(),
    is_link,
    size: if target.is_file() { target.len() } else { 0 },
    modified: target.mtime(),
  }
}

pub fn list(root: &Root, path: &str) -> Result<Listing, AppError> {
  let dir = root.resolve(path)?;
  let entries = fs::read_dir(root.real(&dir))?
    .flatten()
    .filter_map(|item| {
      let name = item.file_name().to_string_lossy().into_owned();
      let meta = item.metadata().ok()?;
      Some(entry(root, &dir.join(&name), name, meta))
    })
    .collect();
  Ok(Listing {
    path: dir.to_string_lossy().into_owned(),
    read_only: root.is_read_only(&dir),
    entries,
  })
}

/// Abre só arquivo comum. O NONBLOCK evita travar em um FIFO.
pub fn open_regular(real: &Path) -> Result<File, AppError> {
  if !fs::metadata(real)?.is_file() {
    return Err(AppError::Invalid("Não é um arquivo comum"));
  }
  let file = OpenOptions::new()
    .read(true)
    .custom_flags(OFlags::NONBLOCK.bits() as i32)
    .open(real)?;
  if !file.metadata()?.is_file() {
    return Err(AppError::Invalid("Não é um arquivo comum"));
  }
  Ok(file)
}

/// Deixa o arquivo novo com o dono da pasta. Só muda algo quando o processo é root.
pub fn inherit_owner(real: &Path) {
  if let Some(Ok(meta)) = real.parent().map(fs::metadata)
    && rustix::process::geteuid().is_root()
  {
    let _ = chown(real, Some(meta.uid()), Some(meta.gid()));
  }
}

/// Renomeia sem sobrescrever.
pub fn rename_new(from: &Path, to: &Path) -> io::Result<()> {
  match rustix::fs::renameat_with(CWD, from, CWD, to, RenameFlags::NOREPLACE) {
    // Alguns sistemas de arquivos (NTFS, exFAT) não têm NOREPLACE.
    Err(rustix::io::Errno::INVAL | rustix::io::Errno::NOSYS) => {
      if fs::symlink_metadata(to).is_ok() {
        return Err(io::ErrorKind::AlreadyExists.into());
      }
      fs::rename(from, to)
    }
    result => Ok(result?),
  }
}

/// "foto.jpg" -> "foto (1).jpg"
fn numbered(name: &str, number: u32) -> String {
  match name.rfind('.').filter(|index| *index > 0) {
    Some(index) => format!("{} ({number}){}", &name[..index], &name[index..]),
    None => format!("{name} ({number})"),
  }
}

/// Coloca `from` em `dir` com o nome pedido ou, se já existir, com um número no final.
pub fn place(from: &Path, dir: &Path, name: &str) -> io::Result<PathBuf> {
  for number in 0..1000 {
    let to = match number {
      0 => dir.join(name),
      _ => dir.join(numbered(name, number)),
    };
    match rename_new(from, &to) {
      Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
      result => return result.map(|()| to),
    }
  }
  Err(io::ErrorKind::AlreadyExists.into())
}

pub fn create(root: &Root, dir: &str, name: &str, is_dir: bool) -> Result<(), AppError> {
  let path = root.resolve(dir)?.join(check_name(name)?);
  root.check_write(&path)?;
  let real = root.real(&path);
  if is_dir {
    fs::create_dir(&real)?;
  } else {
    File::create_new(&real)?;
  }
  inherit_owner(&real);
  Ok(())
}

pub fn rename(root: &Root, path: &str, name: &str) -> Result<(), AppError> {
  let from = root.resolve_link(path)?;
  root.check_move(&from)?;
  let to = from.with_file_name(check_name(name)?);
  root.check_write(&to)?;
  Ok(rename_new(&root.real(&from), &root.real(&to))?)
}

fn version(meta: &Metadata) -> String {
  format!("{}.{}-{}", meta.mtime(), meta.mtime_nsec(), meta.len())
}

pub fn read_text(root: &Root, path: &str) -> Result<Text, AppError> {
  let file = open_regular(&root.real(&root.resolve(path)?))?;
  let meta = file.metadata()?;
  let mut content = Vec::new();
  file.take(MAX_TEXT + 1).read_to_end(&mut content)?;
  if content.len() as u64 > MAX_TEXT {
    return Err(AppError::Invalid("O arquivo passa de 1 MB"));
  }
  if content.contains(&0) {
    return Err(AppError::Invalid("O arquivo não é de texto"));
  }
  Ok(Text {
    content: String::from_utf8(content)
      .map_err(|_| AppError::Invalid("O arquivo não é de texto"))?,
    version: version(&meta),
  })
}

/// Grava no mesmo arquivo, sem trocar: mantém dono, permissões e os arquivos que o docker
/// montou um a um.
pub fn save_text(root: &Root, input: &SaveText) -> Result<Text, AppError> {
  let path = root.resolve(&input.path)?;
  root.check_write(&path)?;
  let real = root.real(&path);
  if version(&open_regular(&real)?.metadata()?) != input.version {
    return Err(AppError::Conflict("O arquivo mudou desde que foi aberto"));
  }
  let mut file = OpenOptions::new().write(true).open(&real)?;
  file.write_all(input.content.as_bytes())?;
  file.set_len(input.content.len() as u64)?;
  file.sync_all()?;
  Ok(Text {
    version: version(&file.metadata()?),
    content: String::new(),
  })
}

/// Procura no nome, sem índice. Não segue links e pula os sistemas do kernel.
pub fn search(
  root: &Root,
  path: &str,
  query: &str,
  cancelled: &AtomicBool,
) -> Result<SearchResult, AppError> {
  let query = query.trim().to_lowercase();
  if query.is_empty() {
    return Err(AppError::Invalid("Digite o que procurar"));
  }
  let start = Instant::now();
  let mut found = Vec::new();
  let mut pending = vec![root.resolve(path)?];

  while let Some(dir) = pending.pop() {
    let Ok(items) = fs::read_dir(root.real(&dir)) else {
      continue;
    };
    for item in items.flatten() {
      let stop = cancelled.load(Ordering::Relaxed) || start.elapsed() > MAX_SEARCH;
      if stop || found.len() >= MAX_RESULTS {
        return Ok(SearchResult {
          found,
          partial: true,
        });
      }
      let name = item.file_name().to_string_lossy().into_owned();
      let path = dir.join(&name);
      if item.file_type().is_ok_and(|kind| kind.is_dir()) && !root.is_virtual(&path) {
        pending.push(path.clone());
      }
      if name.to_lowercase().contains(&query)
        && let Ok(meta) = item.metadata()
      {
        found.push(Found {
          path: path.to_string_lossy().into_owned(),
          entry: entry(root, &path, name, meta),
        });
      }
    }
  }
  Ok(SearchResult {
    found,
    partial: false,
  })
}
