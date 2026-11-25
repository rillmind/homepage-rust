use super::{ops, root::Root};
use crate::{auth::hex, error::AppError};
use image::{ImageDecoder, ImageReader, Limits, codecs::jpeg::JpegEncoder};
use sha2::{Digest, Sha256};
use std::{
  fs,
  io::{self, BufReader},
  os::unix::fs::MetadataExt,
  path::{Path, PathBuf},
};

const SIZE: u32 = 240;
const MAX_FILE: u64 = 40 * 1024 * 1024;
const MAX_SIDE: u32 = 12_000;
const MAX_MEMORY: u64 = 160 * 1024 * 1024;
const MAX_CACHE: u64 = 200 * 1024 * 1024;

/// Esvazia o cache se ele passou do limite. Roda ao iniciar.
pub fn trim(dir: &Path) {
  let total: u64 = fs::read_dir(dir)
    .into_iter()
    .flatten()
    .flatten()
    .filter_map(|item| Some(item.metadata().ok()?.len()))
    .sum();
  if total > MAX_CACHE {
    let _ = fs::remove_dir_all(dir);
  }
}

/// Caminho da miniatura em cache, criada se ainda não existir.
pub fn get(root: &Root, cache: &Path, path: &str) -> Result<PathBuf, AppError> {
  let path = root.resolve(path)?;
  let file = ops::open_regular(&root.real(&path))?;
  let meta = file.metadata()?;
  if meta.len() > MAX_FILE {
    return Err(AppError::Invalid("Imagem grande demais"));
  }
  let key = format!(
    "{}\0{}.{}\0{}",
    path.display(),
    meta.mtime(),
    meta.mtime_nsec(),
    meta.len()
  );
  let target = cache.join(hex(&Sha256::digest(key)) + ".jpg");
  if target.exists() {
    return Ok(target);
  }

  let unsupported = |_| AppError::Invalid("Imagem não suportada");
  let mut reader = ImageReader::new(BufReader::new(file)).with_guessed_format()?;
  let mut limits = Limits::default();
  limits.max_image_width = Some(MAX_SIDE);
  limits.max_image_height = Some(MAX_SIDE);
  limits.max_alloc = Some(MAX_MEMORY);
  reader.limits(limits);
  let mut decoder = reader.into_decoder().map_err(unsupported)?;
  let orientation = decoder.orientation().map_err(unsupported)?;
  let mut image = image::DynamicImage::from_decoder(decoder)
    .map_err(unsupported)?
    .thumbnail(SIZE, SIZE);
  image.apply_orientation(orientation);

  let mut bytes = Vec::new();
  image
    .to_rgb8()
    .write_with_encoder(JpegEncoder::new_with_quality(&mut bytes, 75))
    .map_err(|err| AppError::Io(io::Error::other(err)))?;
  fs::create_dir_all(cache)?;
  let temp = target.with_extension("tmp");
  fs::write(&temp, bytes)?;
  fs::rename(&temp, &target)?;
  Ok(target)
}
