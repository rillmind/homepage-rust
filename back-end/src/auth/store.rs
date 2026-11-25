use crate::{error::AppError, storage};
use argon2::{
  Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
  password_hash::{
    SaltString,
    rand_core::{OsRng, RngCore},
  },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
  io,
  path::PathBuf,
  sync::{Arc, Mutex, MutexGuard},
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const SESSION_SECONDS: u64 = 30 * 24 * 3600;
const MAX_SESSIONS: usize = 20;
const MAX_FAILURES: u32 = 5;
const LOCKOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Deserialize, Serialize)]
struct Session {
  token_hash: String,
  expires: u64,
}

#[derive(Clone, Deserialize, Serialize)]
struct Account {
  username: String,
  password_hash: String,
  sessions: Vec<Session>,
}

#[derive(Default)]
struct Failures {
  count: u32,
  blocked_until: Option<Instant>,
}

/// Usuário único e suas sessões. Só o hash da senha e dos tokens vai para o disco.
#[derive(Clone)]
pub struct Auth {
  path: Arc<PathBuf>,
  account: Arc<Mutex<Option<Account>>>,
  failures: Arc<Mutex<Failures>>,
}

fn now() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_or(0, |time| time.as_secs())
}

pub fn hex(bytes: &[u8]) -> String {
  bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn random_id(bytes: usize) -> String {
  let mut buffer = vec![0; bytes];
  OsRng.fill_bytes(&mut buffer);
  hex(&buffer)
}

fn token_hash(token: &str) -> String {
  hex(&Sha256::digest(token))
}

fn hash_password(password: &str) -> Result<String, AppError> {
  let salt = SaltString::generate(&mut OsRng);
  Argon2::default()
    .hash_password(password.as_bytes(), &salt)
    .map(|hash| hash.to_string())
    .map_err(|err| AppError::Io(io::Error::other(err)))
}

fn validate_password(password: &str) -> Result<(), AppError> {
  if !(6..=200).contains(&password.chars().count()) {
    return Err(AppError::Invalid("A senha deve ter de 6 a 200 caracteres"));
  }
  Ok(())
}

impl Account {
  fn verifies(&self, password: &str) -> bool {
    PasswordHash::new(&self.password_hash).is_ok_and(|hash| {
      Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .is_ok()
    })
  }

  fn add_session(&mut self) -> String {
    let token = random_id(32);
    let time = now();
    self.sessions.retain(|session| session.expires > time);
    if self.sessions.len() >= MAX_SESSIONS {
      self.sessions.remove(0);
    }
    self.sessions.push(Session {
      token_hash: token_hash(&token),
      expires: time + SESSION_SECONDS,
    });
    token
  }
}

impl Auth {
  pub fn open(path: PathBuf) -> io::Result<Self> {
    Ok(Self {
      account: Arc::new(Mutex::new(storage::load(&path)?)),
      path: Arc::new(path),
      failures: Arc::default(),
    })
  }

  fn lock(&self) -> MutexGuard<'_, Option<Account>> {
    self.account.lock().unwrap()
  }

  pub fn username(&self) -> Option<String> {
    self.lock().as_ref().map(|account| account.username.clone())
  }

  /// Só funciona enquanto não existe conta.
  pub fn register(&self, username: &str, password: &str) -> Result<String, AppError> {
    let username = username.trim();
    if !(1..=40).contains(&username.chars().count()) {
      return Err(AppError::Invalid("O usuário deve ter de 1 a 40 caracteres"));
    }
    validate_password(password)?;

    let mut current = self.lock();
    if current.is_some() {
      return Err(AppError::Conflict("Já existe uma conta"));
    }
    let mut account = Account {
      username: username.to_string(),
      password_hash: hash_password(password)?,
      sessions: Vec::new(),
    };
    let token = account.add_session();
    storage::save(&self.path, &Some(&account))?;
    *current = Some(account);
    Ok(token)
  }

  /// O lock das falhas fica preso durante a verificação, então as tentativas são em fila
  /// e não dá para disparar várias em paralelo antes do bloqueio.
  fn check(&self, username: &str, password: &str) -> Result<Account, AppError> {
    let mut failures = self.failures.lock().unwrap();
    if failures
      .blocked_until
      .is_some_and(|until| Instant::now() < until)
    {
      return Err(AppError::TooMany(
        "Muitas tentativas. Espere 30 segundos e tente de novo",
      ));
    }
    let account = self.lock().clone();
    let valid =
      account.filter(|account| account.verifies(password) && account.username == username.trim());
    match valid {
      Some(account) => {
        *failures = Failures::default();
        Ok(account)
      }
      None => {
        failures.count += 1;
        if failures.count >= MAX_FAILURES {
          failures.blocked_until = Some(Instant::now() + LOCKOUT);
        }
        Err(AppError::Unauthorized("Usuário ou senha incorretos"))
      }
    }
  }

  fn change<T>(&self, change: impl FnOnce(&mut Account) -> T) -> Result<T, AppError> {
    let mut current = self.lock();
    let mut next = current.clone().ok_or(AppError::NotFound)?;
    let result = change(&mut next);
    storage::save(&self.path, &Some(&next))?;
    *current = Some(next);
    Ok(result)
  }

  pub fn login(&self, username: &str, password: &str) -> Result<String, AppError> {
    self.check(username, password)?;
    self.change(Account::add_session)
  }

  /// Encerra as outras sessões e devolve uma nova.
  pub fn change_password(&self, current: &str, new: &str) -> Result<String, AppError> {
    validate_password(new)?;
    let username = self.username().ok_or(AppError::NotFound)?;
    self.check(&username, current)?;
    let hash = hash_password(new)?;
    self.change(|account| {
      account.password_hash = hash;
      account.sessions.clear();
      account.add_session()
    })
  }

  pub fn logout(&self, token: &str) -> Result<(), AppError> {
    let hash = token_hash(token);
    self.change(|account| account.sessions.retain(|s| s.token_hash != hash))
  }

  pub fn is_valid(&self, token: &str) -> bool {
    let hash = token_hash(token);
    let time = now();
    self.lock().as_ref().is_some_and(|account| {
      account
        .sessions
        .iter()
        .any(|s| s.token_hash == hash && s.expires > time)
    })
  }
}
