use diesel::prelude::*;

use crate::{
  run_sqlite_query,
  sqlite_tests::{models::*, schema::*},
};

#[tokio::test]
async fn queries() {
  let fire_pokemons_by_name: Vec<String> = run_sqlite_query(|conn| {
    pokemon_types::table
      .inner_join(types::table)
      .inner_join(pokemons::table)
      // We can directly use the enum to filter by variant name
      .filter(types::name.eq(Types::Fire))
      .select(pokemons::name)
      .limit(5)
      .load(conn)
  })
  .await
  .unwrap();

  let fire_pokemons_by_id: Vec<String> = run_sqlite_query(|conn| {
    pokemon_types::table
      .inner_join(types::table)
      .inner_join(pokemons::table)
      // Or we can also map by ID.
      // In this case, we use `TypesId` because we created a double mapping
      .filter(types::id.eq(TypesId::Fire))
      .select(pokemons::name)
      .limit(5)
      .load(conn)
  })
  .await
  .unwrap();

  assert_eq!(fire_pokemons_by_name.len(), 5);
  assert_eq!(fire_pokemons_by_id.len(), 5);

  let fire_pokemons = [
    "Charmander".to_string(),
    "Charmeleon".to_string(),
    "Charizard".to_string(),
    "Vulpix".to_string(),
    "Ninetales".to_string(),
  ];

  for pokemon in &fire_pokemons {
    assert!(fire_pokemons_by_name.contains(pokemon));
    assert!(fire_pokemons_by_id.contains(pokemon));
  }
}
