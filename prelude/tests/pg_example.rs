mod pg_data;

use pg_data::{models::*, postgres_testing_callback, run_pg_query, schema::*};

#[tokio::test]
async fn pg_queries() {
  use diesel::prelude::*;

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

      let selected_row = pokemons::table
        .select(Pokemon::as_select())
        .filter(pokemons::type_.eq(PokemonTypes::Fire))
        .get_result(conn)
        .unwrap();

      assert_eq!(new_row, selected_row);

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
