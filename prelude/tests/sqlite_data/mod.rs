pub mod models;
pub mod schema;

use std::{env, error::Error, time::Duration};

use deadpool_diesel::{
  sqlite::{Hook, HookError, Manager as SqliteManager, Pool as SqlitePool},
  Runtime,
};
use deadpool_sync::SyncWrapper;
use diesel::{prelude::*, SqliteConnection};
use dotenvy::dotenv;
use tokio::sync::OnceCell;

static SQLITE_POOL: OnceCell<deadpool_diesel::sqlite::Pool> = OnceCell::const_new();

pub async fn run_sqlite_query<T: Send + 'static>(
  callback: impl FnOnce(&mut SqliteConnection) -> QueryResult<T> + Send + 'static,
) -> Result<T, Box<dyn Error>> {
  Ok(
    SQLITE_POOL
      .get_or_init(|| async { create_sqlite_pool() })
      .await
      .get()
      .await
      .expect("Could not get a connection")
      .interact(callback)
      .await??,
  )
}

pub fn create_sqlite_pool() -> deadpool_diesel::sqlite::Pool {
  dotenv().ok();

  let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

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
    .expect("could not build the connection pool")
}

async fn connection_setup(conn: &mut SyncWrapper<SqliteConnection>) -> Result<(), HookError> {
  let _ = conn
    .interact(move |conn| {
      diesel::sql_query("PRAGMA busy_timeout = 2000;").execute(conn)?;
      diesel::sql_query("PRAGMA journal_mode = WAL;").execute(conn)?;
      QueryResult::Ok(())
    })
    .await;

  Ok(())
}
