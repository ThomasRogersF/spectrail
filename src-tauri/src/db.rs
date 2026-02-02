use rusqlite::{Connection};
use tauri::{AppHandle, Manager};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
  #[error("tauri error: {0}")]
  Tauri(#[from] tauri::Error),
  #[error("sqlite error: {0}")]
  Sqlite(#[from] rusqlite::Error),
  #[error("io error: {0}")]
  Io(#[from] std::io::Error),
}

pub struct DbPaths {
  pub db_path: std::path::PathBuf,
  pub migrations_dir: std::path::PathBuf,
}

pub fn paths(app: &AppHandle) -> Result<DbPaths, DbError> {
  let app_data = app.path().app_data_dir().map_err(DbError::Tauri)?;
  std::fs::create_dir_all(&app_data)?;
  let db_path = app_data.join("spectrail.sqlite");

  // migrations live in the app bundle during runtime; for dev we copy from repo root.
  // This scaffold keeps SQL in src-tauri/migrations and embeds it at compile time in commands.
  let migrations_dir = app_data.join("migrations");
  std::fs::create_dir_all(&migrations_dir)?;

  Ok(DbPaths { db_path, migrations_dir })
}

pub fn connect(app: &AppHandle) -> Result<Connection, DbError> {
  let p = paths(app)?;
  let conn = Connection::open(p.db_path)?;
  Ok(conn)
}

pub fn init_db(app: &AppHandle) -> Result<(), DbError> {
  let conn = connect(app)?;
  // Apply migrations in order. Each uses IF NOT EXISTS for idempotency.
  let init_sql = include_str!("../migrations/001_init.sql");
  conn.execute_batch(init_sql)?;
  let settings_sql = include_str!("../migrations/002_settings.sql");
  conn.execute_batch(settings_sql)?;
  Ok(())
}
