use std::{env, time::Duration};

use deadpool_diesel::{
  sqlite::{Hook, HookError, Manager, Pool},
  Runtime,
};
use deadpool_sync::SyncWrapper;
use diesel::{prelude::*, SqliteConnection};
use dotenvy::dotenv;
use tokio::sync::OnceCell;

use crate::{
  models::*,
  schema::{pokemon_types, pokemons, types},
};

pub mod models;
pub mod schema;

static POOL: OnceCell<deadpool_diesel::sqlite::Pool> = OnceCell::const_new();

pub async fn testing_callback(
  callback: impl FnOnce(&mut SqliteConnection) + std::marker::Send + 'static,
) {
  POOL
    .get_or_init(|| async { create_pool(true) })
    .await
    .get()
    .await
    .expect("Could not get a connection")
    .interact(callback)
    .await
    .expect("Testing outcome was unsuccessful")
}

pub fn create_pool(is_test: bool) -> deadpool_diesel::sqlite::Pool {
  dotenv().ok();

  let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

  let manager = Manager::new(database_url, Runtime::Tokio1);

  let mut builder = Pool::builder(manager);

  builder = if is_test {
    builder.max_size(1)
  } else {
    builder.max_size(8)
  };

  builder
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
      // this corresponds to 2 seconds
      // if we ever see errors regarding busy_timeout in production
      // we might want to consider to increase this time
      diesel::sql_query("PRAGMA busy_timeout = 2000;").execute(conn)?;
      // better write-concurrency
      diesel::sql_query("PRAGMA journal_mode = WAL;").execute(conn)?;
      // fsync only in critical moments
      diesel::sql_query("PRAGMA synchronous = NORMAL;").execute(conn)?;
      // write WAL changes back every 1000 pages, for an in average 1MB WAL file. May affect readers if number is increased
      diesel::sql_query("PRAGMA wal_autocheckpoint = 1000;").execute(conn)?;
      // free some space by truncating possibly massive WAL files from the last run
      diesel::sql_query("PRAGMA wal_checkpoint(TRUNCATE);").execute(conn)?;
      // maximum size of the WAL file, corresponds to 64MB
      diesel::sql_query("PRAGMA journal_size_limit = 67108864;").execute(conn)?;
      // maximum size of the internal mmap pool. Corresponds to 128MB, matches postgres default settings
      diesel::sql_query("PRAGMA mmap_size = 134217728;").execute(conn)?;
      // maximum number of database disk pages that will be hold in memory. Corresponds to ~8MB
      diesel::sql_query("PRAGMA cache_size = 2000;").execute(conn)?;
      //enforce foreign keys
      diesel::sql_query("PRAGMA foreign_keys = ON;").execute(conn)?;
      QueryResult::Ok(())
    })
    .await;

  Ok(())
}

#[allow(dead_code)]
#[derive(Debug)]
struct PokeData {
  pub pokemon: Pokemon,
  pub images: ImageData,
  pub stats: BaseStat,
  pub types: Vec<String>,
}

#[allow(dead_code)]
fn select_pokemon(conn: &mut SqliteConnection) -> Result<(), Box<dyn std::error::Error>> {
  let poke_data: Pokemon = pokemons::table
    .filter(pokemons::id.eq(1))
    .select(Pokemon::as_select())
    .get_result(conn)?;

  let base_stats = BaseStat::belonging_to(&poke_data)
    .select(BaseStat::as_select())
    .get_result(conn)?;
  let img_data = ImageData::belonging_to(&poke_data)
    .select(ImageData::as_select())
    .get_result(conn)?;

  let poke_types = PokemonType::belonging_to(&poke_data)
    .inner_join(types::table)
    .select(types::name)
    .load::<String>(conn)?;

  let complete_data = PokeData {
    pokemon: poke_data,
    stats: base_stats,
    types: poke_types,
    images: img_data,
  };
  println!("Complete data: {:#?}", complete_data);
  Ok(())
}

#[allow(dead_code)]
#[diesel::dsl::auto_type]
fn pokemons_with_types() -> _ {
  pokemon_types::table
    .inner_join(types::table)
    .inner_join(pokemons::table)
}

#[allow(dead_code)]
fn pokemons_by_type(
  conn: &mut SqliteConnection,
  poke_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
  let pokemons_with_types = pokemons_with_types();

  let pokemons = pokemons_with_types
    .filter(types::name.eq(poke_type))
    .select(pokemons::name)
    .limit(5)
    .load::<String>(conn)?;

  println!("{poke_type} pokemons: {:#?}", pokemons);

  Ok(())
}

// #[cfg(test)]
// mod test {
//   use crate::{create_pool, pokemons_by_type};
//
//   #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
//   async fn test_query() {
//     create_pool(true)
//       .get()
//       .await
//       .expect("Could not get a pool")
//       .interact(|conn| {
//         pokemons_by_type(conn, "Grass").expect("Failed query");
//       })
//       .await
//       .expect("Query unsuccessful")
//   }
// }
