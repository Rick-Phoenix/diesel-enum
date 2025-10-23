use diesel_enums::diesel_enum;

use crate::{
  models::{PgTable, PgTypes},
  run_pg_query,
};

#[tokio::test]
async fn you_shall_pass() {
  PgTypes::check_consistency().await.unwrap();
}

mod wrong_casing {
  use super::*;

  #[diesel_enum(conn = crate::postgres_testing_callback, case = "UPPERCASE", name_mapping(name = "pokemon_type", path = crate::pg_schema::sql_types::PokemonType))]
  enum PgTypes {
    Grass,
    Poison,
    Fire,
    Flying,
    Water,
    Bug,
    Normal,
    Electric,
    Ground,
    Fairy,
    Fighting,
    Psychic,
    Rock,
    Steel,
    Ice,
    Ghost,
    Dragon,
    Dark,
  }

  #[tokio::test]
  #[should_panic]
  async fn wrong_casing() {
    PgTypes::check_consistency().await.unwrap();
  }
}

mod missing_db_variant {
  use super::*;

  #[diesel_enum(conn = crate::postgres_testing_callback,  name_mapping(name = "pokemon_type", path = crate::pg_schema::sql_types::PokemonType))]
  enum PgTypes {
    // Grass,
    Poison,
    Fire,
    Flying,
    Water,
    Bug,
    Normal,
    Electric,
    Ground,
    Fairy,
    Fighting,
    Psychic,
    Rock,
    Steel,
    Ice,
    Ghost,
    Dragon,
    Dark,
  }

  #[tokio::test]
  #[should_panic]
  async fn missing_db_variant() {
    PgTypes::check_consistency().await.unwrap();
  }
}

mod extra_variant {
  use super::*;

  #[diesel_enum(conn = crate::postgres_testing_callback,  name_mapping(name = "pokemon_type", path = crate::pg_schema::sql_types::PokemonType))]
  enum PgTypes {
    NotAPokemonType,
    Grass,
    Poison,
    Fire,
    Flying,
    Water,
    Bug,
    Normal,
    Electric,
    Ground,
    Fairy,
    Fighting,
    Psychic,
    Rock,
    Steel,
    Ice,
    Ghost,
    Dragon,
    Dark,
  }

  #[tokio::test]
  #[should_panic]
  async fn extra_variant() {
    PgTypes::check_consistency().await.unwrap();
  }
}

#[tokio::test]
async fn pg_queries() {
  use diesel::prelude::*;

  use crate::pg_schema::*;

  let new_row = PgTable {
    name: "Charizard".to_string(),
    type_: PgTypes::Fire,
  };

  run_pg_query(|conn| {
    Ok(conn.test_transaction(move |conn| -> Result<(), String> {
      let inserted_row: PgTable = diesel::insert_into(pokemon_table::table)
        .values(&new_row)
        .get_result(conn)
        .unwrap();

      assert_eq!(new_row, inserted_row);

      let selected_row: PgTable = pokemon_table::table
        .select(PgTable::as_select())
        .filter(pokemon_table::type_.eq(PgTypes::Fire))
        .get_result(conn)
        .unwrap();

      assert_eq!(new_row, selected_row);

      let updated_row: PgTable =
        diesel::update(pokemon_table::table.filter(pokemon_table::type_.eq(PgTypes::Fire)))
          .set(pokemon_table::type_.eq(PgTypes::Fire))
          .get_result(conn)
          .unwrap();

      assert_eq!(updated_row.type_, PgTypes::Fire);

      let deleted_row =
        diesel::delete(pokemon_table::table.filter(pokemon_table::type_.eq(PgTypes::Fire)))
          .get_result(conn)
          .unwrap();

      assert_eq!(new_row, deleted_row);

      Ok(())
    }))
  })
  .await
  .unwrap();
}
