use std::{env, error::Error, time::Duration};

use deadpool_diesel::{
  postgres::{Manager as PgManager, Pool as PgPool},
  sqlite::{Hook, HookError, Manager as SqliteManager, Pool as SqlitePool},
  Runtime,
};
use deadpool_sync::SyncWrapper;
use diesel::{prelude::*, SqliteConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenvy::dotenv;
use pgtemp::PgTempDB;
use tokio::sync::OnceCell;

const PG_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/pg");

#[cfg(test)]
pub mod pg_tests;
#[cfg(test)]
pub mod sqlite_tests;

static SQLITE_POOL: OnceCell<deadpool_diesel::sqlite::Pool> = OnceCell::const_new();
static POSTGRES_POOL: OnceCell<deadpool_diesel::postgres::Pool> = OnceCell::const_new();

#[cfg(test)]
pub async fn postgres_testing_callback(
  callback: impl FnOnce(&mut PgConnection) -> Result<(), diesel_enums::DbEnumError>
    + std::marker::Send
    + 'static,
) -> Result<(), diesel_enums::DbEnumError> {
  POSTGRES_POOL
    .get_or_init(|| async { create_pg_pool().await })
    .await
    .get()
    .await
    .expect("Could not get a connection")
    .interact(callback)
    .await
    .expect("Testing outcome was unsuccessful")
}

pub async fn run_pg_query<T: Send + 'static>(
  callback: impl FnOnce(&mut PgConnection) -> QueryResult<T> + Send + 'static,
) -> Result<T, Box<dyn Error>> {
  Ok(
    POSTGRES_POOL
      .get_or_init(|| async { create_pg_pool().await })
      .await
      .get()
      .await
      .expect("Could not get a connection")
      .interact(callback)
      .await??,
  )
}

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

// Needs to be put here to avoid being dropped earlier
static PG_TEMP: OnceCell<PgTempDB> = OnceCell::const_new();

pub async fn create_pg_pool() -> deadpool_diesel::postgres::Pool {
  let db = PG_TEMP
    .get_or_init(async || PgTempDB::async_new().await)
    .await;

  let url = db.connection_uri();

  let manager = PgManager::new(url, Runtime::Tokio1);

  let pool = PgPool::builder(manager)
    .max_size(1)
    .runtime(Runtime::Tokio1)
    .build()
    .expect("could not build the postgres connection pool");

  pool
    .get()
    .await
    .unwrap()
    .interact(|conn| {
      conn
        .run_pending_migrations(PG_MIGRATIONS)
        .expect("Failed to run migrations");
    })
    .await
    .unwrap();

  pool
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
