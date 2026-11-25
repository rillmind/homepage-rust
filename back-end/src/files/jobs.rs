use crate::error::AppError;
use serde::Serialize;
use std::{
  collections::BTreeMap,
  io,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
};

const KEEP_FINISHED: usize = 20;

#[derive(Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
  Waiting,
  Running,
  Done,
  Failed,
  Cancelled,
}

#[derive(Default)]
pub struct Progress {
  pub done: AtomicU64,
  pub total: AtomicU64,
  cancelled: AtomicBool,
}

impl Progress {
  pub fn add(&self, bytes: u64) {
    self.done.fetch_add(bytes, Ordering::Relaxed);
  }

  pub fn check(&self) -> io::Result<()> {
    if self.cancelled.load(Ordering::Relaxed) {
      return Err(io::ErrorKind::Interrupted.into());
    }
    Ok(())
  }
}

struct Job {
  label: String,
  state: State,
  error: Option<&'static str>,
  progress: Arc<Progress>,
}

#[derive(Serialize)]
pub struct JobView {
  id: u64,
  label: String,
  state: State,
  error: Option<&'static str>,
  done: u64,
  total: u64,
}

/// Trabalhos longos em segundo plano, um por vez para não disputar o disco.
#[derive(Clone, Default)]
pub struct Jobs {
  list: Arc<Mutex<BTreeMap<u64, Job>>>,
  next: Arc<AtomicU64>,
  turn: Arc<tokio::sync::Mutex<()>>,
}

impl Jobs {
  pub fn start(
    &self,
    label: String,
    work: impl FnOnce(&Progress) -> Result<(), AppError> + Send + 'static,
  ) -> u64 {
    let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
    let progress = Arc::new(Progress::default());
    {
      let mut list = self.list.lock().unwrap();
      list.insert(
        id,
        Job {
          label,
          state: State::Waiting,
          error: None,
          progress: progress.clone(),
        },
      );
      let finished: Vec<u64> = list
        .iter()
        .filter(|(_, job)| !matches!(job.state, State::Waiting | State::Running))
        .map(|(id, _)| *id)
        .collect();
      for id in finished.iter().rev().skip(KEEP_FINISHED) {
        list.remove(id);
      }
    }

    let jobs = self.clone();
    tokio::spawn(async move {
      let _turn = jobs.turn.lock().await;
      jobs.set(id, State::Running, None);
      let result = crate::error::blocking(move || {
        progress.check()?;
        work(&progress)
      })
      .await;
      match result {
        Ok(()) => jobs.set(id, State::Done, None),
        Err(AppError::Io(err)) if err.kind() == io::ErrorKind::Interrupted => {
          jobs.set(id, State::Cancelled, None)
        }
        Err(err) => {
          eprintln!("Trabalho {id}: {err:?}");
          jobs.set(id, State::Failed, Some(err.message()))
        }
      }
    });
    id
  }

  fn set(&self, id: u64, state: State, error: Option<&'static str>) {
    if let Some(job) = self.list.lock().unwrap().get_mut(&id) {
      job.state = state;
      job.error = error;
    }
  }

  pub fn cancel(&self, id: u64) -> Result<(), AppError> {
    let list = self.list.lock().unwrap();
    let job = list.get(&id).ok_or(AppError::NotFound)?;
    job.progress.cancelled.store(true, Ordering::Relaxed);
    Ok(())
  }

  pub fn list(&self) -> Vec<JobView> {
    let list = self.list.lock().unwrap();
    list
      .iter()
      .map(|(id, job)| JobView {
        id: *id,
        label: job.label.clone(),
        state: job.state,
        error: job.error,
        done: job.progress.done.load(Ordering::Relaxed),
        total: job.progress.total.load(Ordering::Relaxed),
      })
      .collect()
  }
}
