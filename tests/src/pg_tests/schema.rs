// @generated automatically by Diesel CLI.

pub mod sql_types {
  use diesel::query_builder::QueryId;

  #[derive(diesel::sql_types::SqlType, QueryId, Debug)]
  #[diesel(postgres_type(name = "pokemon_type"))]
  pub struct PokemonType;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::PokemonType;

    pokemons (name) {
        name -> Text,
        #[sql_name = "type"]
        type_ -> PokemonType,
    }
}
