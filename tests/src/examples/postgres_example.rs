use diesel::{prelude::*, PgConnection};

use crate::pg_tests::{
  models::{Pokemon, PokemonTypes},
  schema::pokemons,
};

pub fn select_fire_pokemons(conn: &mut PgConnection) -> Vec<Pokemon> {
  pokemons::table
    .select(Pokemon::as_select())
    .filter(pokemons::type_.eq(PokemonTypes::Fire))
    .get_results(conn)
    .unwrap()
}
