use std::env;

use deadpool_diesel::{
  postgres::{Manager as PgManager, Pool as PgPool},
  Runtime,
};
use diesel::prelude::*;
use diesel_enums::DbEnumError;
use dotenvy::dotenv;
use tokio::sync::OnceCell;

static POSTGRES_POOL: OnceCell<deadpool_diesel::postgres::Pool> = OnceCell::const_new();

pub async fn postgres_runner(
  callback: impl FnOnce(&mut PgConnection) -> Result<(), DbEnumError> + std::marker::Send + 'static,
) -> Result<(), DbEnumError> {
  POSTGRES_POOL
    .get_or_init(|| async { create_pg_pool() })
    .await
    .get()
    .await
    .expect("Failed to get a connection to the Postgres database")
    .interact(callback)
    .await
    .expect("Postgres testing pool thread crashed")
}

#[track_caller]
fn create_pg_pool() -> deadpool_diesel::postgres::Pool {
  dotenv().ok();

  let database_url = env::var("DATABASE_URL")
    .expect("Failed to set up testing pool for Postgres: DATABASE_URL is not set");

  let manager = PgManager::new(database_url, Runtime::Tokio1);

  PgPool::builder(manager)
    .max_size(1)
    .runtime(Runtime::Tokio1)
    .build()
    .expect("Failed to create the connection pool for Postgres")
}
