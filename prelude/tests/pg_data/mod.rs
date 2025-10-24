pub mod models;
pub mod schema;

use std::error::Error;

use deadpool_diesel::{
  postgres::{Manager as PgManager, Pool as PgPool},
  Runtime,
};
use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use pgtemp::PgTempDB;
use tokio::sync::OnceCell;

const PG_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/pg");

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
