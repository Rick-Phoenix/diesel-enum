mod pg_data;

use diesel_enums::{diesel_enum, ErrorKind};
use pg_data::{models::*, postgres_testing_callback, run_pg_query, schema::*};

#[tokio::test]
async fn you_shall_pass() {
  PokemonTypes::check_consistency().await.unwrap();
}

mod wrong_casing {

  use super::*;

  #[diesel_enum(conn = postgres_testing_callback, skip_test, case = "UPPERCASE", name_mapping(name = "pokemon_type", path = sql_types::PokemonType))]
  enum PokemonTypes {
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
  async fn wrong_casing() {
    let errors = PokemonTypes::check_consistency().await.unwrap_err().errors;

    assert_eq!(errors.len(), 2);

    assert!(errors.iter().any(|e| {
      if let ErrorKind::MissingFromDb(items) = e {
        items.len() == 18
      } else if let ErrorKind::MissingFromRustEnum(items) = e {
        items.len() == 18
      } else {
        false
      }
    }));
  }
}

mod missing_db_variant {
  use super::*;

  #[diesel_enum(conn = postgres_testing_callback, skip_test,  name_mapping(name = "pokemon_type", path = sql_types::PokemonType))]
  enum PokemonTypes {
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
  async fn missing_db_variant() {
    let errors = PokemonTypes::check_consistency().await.unwrap_err().errors;

    assert_eq!(errors.len(), 1);

    let e = errors.first().unwrap();

    if let ErrorKind::MissingFromRustEnum(items) = e {
      assert!(items.len() == 1);
      assert_eq!(items[0], "grass");
    } else {
      panic!();
    };
  }
}

mod extra_variant {
  use super::*;

  #[diesel_enum(conn = postgres_testing_callback, skip_test, name_mapping(name = "pokemon_type", path = sql_types::PokemonType))]
  enum PokemonTypes {
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
  async fn extra_variant() {
    let errors = PokemonTypes::check_consistency().await.unwrap_err().errors;

    assert_eq!(errors.len(), 1);

    let e = errors.first().unwrap();

    if let ErrorKind::MissingFromDb(items) = e {
      assert!(items.len() == 1);
      assert_eq!(items[0], "not_a_pokemon_type");
    } else {
      panic!();
    };
  }
}

#[tokio::test]
async fn pg_queries() {
  use diesel::prelude::*;

  pub fn select_fire_pokemons(conn: &mut PgConnection) -> Vec<Pokemon> {
    pokemons::table
      .select(Pokemon::as_select())
      .filter(pokemons::type_.eq(PokemonTypes::Fire))
      .get_results(conn)
      .unwrap()
  }

  run_pg_query(|conn| {
    let _: () = conn.test_transaction(|conn| -> Result<(), String> {
      let new_row = Pokemon {
        name: "Charizard".to_string(),
        type_: PokemonTypes::Fire,
      };

      let inserted_row: Pokemon = diesel::insert_into(pokemons::table)
        .values(&new_row)
        .get_result(conn)
        .unwrap();

      assert_eq!(new_row, inserted_row);

      let rows = select_fire_pokemons(conn);
      let selected_row = rows.first().unwrap();

      assert_eq!(&new_row, selected_row);

      let updated_row: Pokemon =
        diesel::update(pokemons::table.filter(pokemons::type_.eq(PokemonTypes::Fire)))
          .set(pokemons::type_.eq(PokemonTypes::Fire))
          .get_result(conn)
          .unwrap();

      assert_eq!(updated_row.type_, PokemonTypes::Fire);

      let deleted_row =
        diesel::delete(pokemons::table.filter(pokemons::type_.eq(PokemonTypes::Fire)))
          .get_result(conn)
          .unwrap();

      assert_eq!(new_row, deleted_row);

      Ok(())
    });
    Ok(())
  })
  .await
  .unwrap();
}
