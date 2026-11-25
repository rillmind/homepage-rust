use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
  Http,
  Https,
}

/// O que o usuário preenche. O serviço roda nesta máquina, por isso só a porta.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ServiceInput {
  pub name: String,
  #[serde(default)]
  pub icon: String,
  pub protocol: Protocol,
  pub port: u16,
  pub category: String,
  #[serde(default)]
  pub description: String,
  /// Nome do contêiner. Vazio, é procurado pela porta.
  #[serde(default)]
  pub container: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Service {
  pub id: u64,
  #[serde(flatten)]
  pub input: ServiceInput,
}

impl ServiceInput {
  pub fn validate(mut self) -> Result<Self, AppError> {
    for field in [
      &mut self.name,
      &mut self.icon,
      &mut self.category,
      &mut self.description,
      &mut self.container,
    ] {
      *field = field.trim().to_string();
    }
    let too_long = |text: &str, max| text.chars().count() > max;

    if self.name.is_empty() || too_long(&self.name, 60) {
      return Err(AppError::Invalid("O nome deve ter de 1 a 60 caracteres"));
    }
    if self.category.is_empty() || too_long(&self.category, 30) {
      return Err(AppError::Invalid(
        "A categoria deve ter de 1 a 30 caracteres",
      ));
    }
    if too_long(&self.description, 120) {
      return Err(AppError::Invalid("A descrição deve ter até 120 caracteres"));
    }
    let name = |c: char| c.is_ascii_alphanumeric() || "_.-".contains(c);
    if too_long(&self.container, 128) || !self.container.chars().all(name) {
      return Err(AppError::Invalid("Nome de contêiner inválido"));
    }
    if self.port == 0 {
      return Err(AppError::Invalid("A porta deve ser de 1 a 65535"));
    }
    let http = self.icon.starts_with("http://") || self.icon.starts_with("https://");
    if too_long(&self.icon, 500) || !(self.icon.is_empty() || http) {
      return Err(AppError::Invalid("O ícone deve ser uma URL http ou https"));
    }
    Ok(self)
  }
}
