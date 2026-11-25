use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
  pub name: String,
  /// Um link para pasta também é pasta.
  pub is_dir: bool,
  pub is_link: bool,
  pub size: u64,
  /// Segundos desde 1970.
  pub modified: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
  pub path: String,
  pub read_only: bool,
  pub entries: Vec<Entry>,
}

#[derive(Serialize)]
pub struct Found {
  pub path: String,
  #[serde(flatten)]
  pub entry: Entry,
}

#[derive(Serialize)]
pub struct SearchResult {
  pub found: Vec<Found>,
  /// Parou no limite de resultados ou de tempo.
  pub partial: bool,
}

#[derive(Deserialize)]
pub struct PathQuery {
  pub path: String,
}

#[derive(Deserialize)]
pub struct DownloadQuery {
  pub path: String,
  #[serde(default)]
  pub inline: bool,
}

#[derive(Deserialize)]
pub struct SearchQuery {
  pub path: String,
  pub q: String,
}

#[derive(Deserialize)]
pub struct NewItem {
  pub dir: String,
  pub name: String,
}

#[derive(Deserialize)]
pub struct Rename {
  pub path: String,
  pub name: String,
}

#[derive(Deserialize)]
pub struct Items {
  pub items: Vec<String>,
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
  Copy,
  Move,
}

#[derive(Deserialize)]
pub struct Transfer {
  pub items: Vec<String>,
  pub to: String,
  pub mode: Mode,
}

#[derive(Serialize)]
pub struct Text {
  pub content: String,
  /// Versão lida, para não salvar por cima de uma mudança feita por fora.
  pub version: String,
}

#[derive(Deserialize)]
pub struct SaveText {
  pub path: String,
  pub content: String,
  pub version: String,
}
