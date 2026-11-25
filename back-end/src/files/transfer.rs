use super::{
  jobs::Progress,
  models::{Mode, Transfer},
  ops,
  root::Root,
};
use crate::{auth::random_id, error::AppError};
use std::{
  ffi::OsString,
  fs::{self, File, FileTimes, Metadata},
  io::{self, Read, Write},
  os::unix::{
    ffi::OsStringExt,
    fs::{MetadataExt, lchown, symlink},
  },
  path::{Path, PathBuf},
  sync::atomic::Ordering,
};

const BUFFER: usize = 1024 * 1024;

/// Item incompleto, apagado se o trabalho falhar ou for cancelado no meio.
pub struct Partial(pub PathBuf);

impl Partial {
  pub fn new(dir: &Path) -> Self {
    Self(dir.join(format!(".homepage-part-{}", random_id(8))))
  }
}

impl Drop for Partial {
  fn drop(&mut self) {
    remove(&self.0).ok();
  }
}

pub fn remove(path: &Path) -> io::Result<()> {
  if fs::symlink_metadata(path)?.is_dir() {
    fs::remove_dir_all(path)
  } else {
    fs::remove_file(path)
  }
}

pub fn size(path: &Path, progress: &Progress) -> io::Result<u64> {
  progress.check()?;
  let meta = fs::symlink_metadata(path)?;
  if !meta.is_dir() {
    return Ok(if meta.is_file() { meta.len() } else { 0 });
  }
  let mut total = 0;
  for item in fs::read_dir(path)? {
    total += size(&item?.path(), progress)?;
  }
  Ok(total)
}

fn copy_file(from: &Path, to: &Path, progress: &Progress) -> Result<(), AppError> {
  let mut source = ops::open_regular(from)?;
  let mut target = File::create_new(to)?;
  // Em btrfs e xfs, no mesmo disco, a cópia é instantânea.
  if rustix::fs::ioctl_ficlone(&target, &source).is_ok() {
    progress.add(source.metadata()?.len());
    return Ok(());
  }
  let mut buffer = vec![0; BUFFER];
  loop {
    progress.check()?;
    let count = source.read(&mut buffer)?;
    if count == 0 {
      return Ok(());
    }
    target.write_all(&buffer[..count])?;
    progress.add(count as u64);
  }
}

/// Mantém o que der do original. Sem ser root, não dá para manter o dono.
fn copy_attributes(to: &Path, meta: &Metadata) -> io::Result<()> {
  let _ = lchown(to, Some(meta.uid()), Some(meta.gid()));
  if meta.is_symlink() {
    return Ok(());
  }
  if let Ok(modified) = meta.modified() {
    let times = FileTimes::new().set_modified(modified);
    let _ = File::open(to).and_then(|file| file.set_times(times));
  }
  fs::set_permissions(to, meta.permissions())
}

fn copy_tree(from: &Path, to: &Path, progress: &Progress) -> Result<(), AppError> {
  progress.check()?;
  let meta = fs::symlink_metadata(from)?;
  if meta.is_symlink() {
    symlink(fs::read_link(from)?, to)?;
  } else if meta.is_dir() {
    fs::create_dir(to)?;
    for item in fs::read_dir(from)? {
      let item = item?;
      copy_tree(&item.path(), &to.join(item.file_name()), progress)?;
    }
  } else {
    copy_file(from, to, progress)?;
  }
  Ok(copy_attributes(to, &meta)?)
}

fn crosses_device(err: &io::Error) -> bool {
  err.kind() == io::ErrorKind::CrossesDevices
}

/// Decodifica os \\040 do mountinfo.
fn unescape(text: &str) -> PathBuf {
  let bytes = text.as_bytes();
  let mut path = Vec::new();
  let mut index = 0;
  while index < bytes.len() {
    let octal = text
      .get(index + 1..index + 4)
      .filter(|_| bytes[index] == b'\\')
      .and_then(|digits| u8::from_str_radix(digits, 8).ok());
    path.push(octal.unwrap_or(bytes[index]));
    index += if octal.is_some() { 4 } else { 1 };
  }
  PathBuf::from(OsString::from_vec(path))
}

fn holds_mount(mountinfo: &str, path: &Path) -> bool {
  mountinfo
    .lines()
    .filter_map(|line| line.split(' ').nth(4))
    .any(|mount| unescape(mount).starts_with(path))
}

/// Uma pasta com outro disco montado dentro não pode ser apagada: levaria junto o conteúdo
/// dele. O mountinfo também pega os bind mounts, que o número do dispositivo não mostra.
pub fn check_mounts(real: &Path) -> Result<(), AppError> {
  if holds_mount(&fs::read_to_string("/proc/self/mountinfo")?, real) {
    return Err(AppError::Forbidden(
      "Há outro disco montado dentro desta pasta",
    ));
  }
  Ok(())
}

/// Copia e só depois apaga o original.
fn move_across(from: &Path, dir: &Path, name: &str, progress: &Progress) -> Result<(), AppError> {
  check_mounts(from)?;
  let partial = Partial::new(dir);
  copy_tree(from, &partial.0, progress)?;
  ops::place(&partial.0, dir, name)?;
  Ok(remove(from)?)
}

/// Valida tudo antes de começar, para o erro aparecer na hora.
pub fn prepare(root: &Root, input: &Transfer) -> Result<(Vec<PathBuf>, PathBuf), AppError> {
  let dir = root.resolve(&input.to)?;
  root.check_write(&dir)?;
  if !fs::metadata(root.real(&dir))?.is_dir() {
    return Err(AppError::Invalid("O destino não é uma pasta"));
  }
  let mut items = Vec::new();
  for item in &input.items {
    let path = root.resolve_link(item)?;
    if input.mode == Mode::Move {
      root.check_move(&path)?;
    }
    if path.file_name().is_none() || root.is_virtual(&path) {
      return Err(AppError::Forbidden("Este item é protegido"));
    }
    if dir.starts_with(&path) {
      return Err(AppError::Invalid("Uma pasta não cabe dentro dela mesma"));
    }
    if input.mode == Mode::Move && path.parent() == Some(&dir) {
      return Err(AppError::Invalid("O item já está nesta pasta"));
    }
    items.push(path);
  }
  Ok((items, dir))
}

pub fn run(
  root: &Root,
  items: &[PathBuf],
  dir: &Path,
  mode: Mode,
  progress: &Progress,
) -> Result<(), AppError> {
  let real_dir = root.real(dir);
  let mut pending = Vec::new();

  // Mover no mesmo disco é só renomear, sem precisar somar tamanhos.
  for item in items {
    progress.check()?;
    let real = root.real(item);
    let name = item
      .file_name()
      .unwrap_or_default()
      .to_string_lossy()
      .into_owned();
    if mode == Mode::Move {
      match ops::place(&real, &real_dir, &name) {
        Err(err) if crosses_device(&err) => {}
        result => {
          result?;
          continue;
        }
      }
    }
    pending.push((real, name));
  }

  let mut total = 0;
  for (real, _) in &pending {
    total += size(real, progress)?;
  }
  progress.total.store(total, Ordering::Relaxed);

  for (real, name) in pending {
    if mode == Mode::Move {
      move_across(&real, &real_dir, &name, progress)?;
    } else {
      let partial = Partial::new(&real_dir);
      copy_tree(&real, &partial.0, progress)?;
      ops::place(&partial.0, &real_dir, &name)?;
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn detecta_disco_montado_dentro_da_pasta() {
    let info = "36 35 8:1 / /media/meu\\040hd rw - ext4 /dev/sdb1 rw\n";
    assert!(holds_mount(info, Path::new("/media")));
    assert!(holds_mount(info, Path::new("/media/meu hd")));
    assert!(!holds_mount(info, Path::new("/media/meu")));
    assert!(!holds_mount(info, Path::new("/media/meu hd/filmes")));
  }
}
