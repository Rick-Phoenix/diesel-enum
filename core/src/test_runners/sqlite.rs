use std::{env, time::Duration};

use deadpool_diesel::{
  sqlite::{Hook, HookError, Manager as SqliteManager, Pool as SqlitePool},
  Runtime,
};
use deadpool_sync::SyncWrapper;
use diesel::{prelude::*, SqliteConnection};
use dotenvy::dotenv;
use tokio::sync::OnceCell;

use crate::DbEnumError;

static SQLITE_POOL: OnceCell<deadpool_diesel::sqlite::Pool> = OnceCell::const_new();

pub async fn sqlite_runner(
  callback: impl FnOnce(&mut SqliteConnection) -> Result<(), DbEnumError> + std::marker::Send + 'static,
) -> Result<(), DbEnumError> {
  SQLITE_POOL
    .get_or_init(|| async { create_sqlite_pool() })
    .await
    .get()
    .await
    .expect("Failed to get a connection to the SQLite database")
    .interact(callback)
    .await
    .expect("Sqlite testing pool thread crashed")
}

#[track_caller]
fn create_sqlite_pool() -> deadpool_diesel::sqlite::Pool {
  dotenv().ok();

  let database_url = env::var("DATABASE_URL")
    .expect("Failed to set up testing pool for SQLite: DATABASE_URL is not set");

  let manager = SqliteManager::new(database_url, Runtime::Tokio1);

  SqlitePool::builder(manager)
    .max_size(1)
    .runtime(Runtime::Tokio1)
    .wait_timeout(Some(Duration::from_secs(5)))
    .create_timeout(Some(Duration::from_secs(5)))
    .recycle_timeout(Some(Duration::from_secs(2)))
    .post_create(Hook::async_fn(move |conn, _metrics| {
      Box::pin(connection_setup(conn))
    }))
    .build()
    .expect("Failed to build the connection pool for SQLite")
}

async fn connection_setup(conn: &mut SyncWrapper<SqliteConnection>) -> Result<(), HookError> {
  let _ = conn
    .interact(move |conn| {
      diesel::sql_query("PRAGMA synchronous = NORMAL;").execute(conn)?;
      diesel::sql_query("PRAGMA busy_timeout = 2000;").execute(conn)?;
      diesel::sql_query("PRAGMA journal_mode = WAL;").execute(conn)?;
      diesel::sql_query("PRAGMA mmap_size = 134217728;").execute(conn)?;
      diesel::sql_query("PRAGMA cache_size = 2000;").execute(conn)?;
      QueryResult::Ok(())
    })
    .await;

  Ok(())
}
