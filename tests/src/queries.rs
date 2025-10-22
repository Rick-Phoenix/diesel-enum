use diesel::prelude::*;

use crate::{
  models::{Types, TypesId},
  run_query,
  schema::{pokemon_types, pokemons, types},
};

#[tokio::test]
async fn select() {
  let fire_pokemons_by_id: Vec<String> = run_query(|conn| {
    pokemon_types::table
      .inner_join(types::table)
      .inner_join(pokemons::table)
      .filter(types::id.eq(TypesId::Fire))
      .select(pokemons::name)
      .limit(5)
      .load(conn)
  })
  .await
  .unwrap();

  assert_eq!(fire_pokemons_by_id.len(), 5);

  let fire_pokemons_by_name: Vec<String> = run_query(|conn| {
    pokemon_types::table
      .inner_join(types::table)
      .inner_join(pokemons::table)
      .filter(types::name.eq(Types::Fire))
      .select(pokemons::name)
      .limit(5)
      .load(conn)
  })
  .await
  .unwrap();

  assert_eq!(fire_pokemons_by_name.len(), 5);
}

#[tokio::test]
async fn modify() {
  run_query(|conn| {
    conn.transaction(|conn| {
      diesel::delete(types::table.filter(types::id.eq(TypesId::Poison))).execute(conn)?;

      diesel::insert_into(types::table)
        .values((types::id.eq(TypesId::Poison), types::name.eq(Types::Poison)))
        .execute(conn)?;

      let result: (TypesId, Types) =
        diesel::update(types::table.filter(types::id.eq(TypesId::Poison)))
          .set((types::id.eq(TypesId::Poison), types::name.eq(Types::Poison)))
          .get_result(conn)?;

      assert_eq!((TypesId::Poison, Types::Poison), result);

      Ok(())
    })
  })
  .await
  .unwrap();
}
