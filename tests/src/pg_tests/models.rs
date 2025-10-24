use diesel::prelude::*;
use diesel_enums::diesel_enum;

use super::schema::*;

#[derive(Queryable, Selectable, Debug, Insertable, PartialEq, Clone)]
pub struct Pokemon {
  pub name: String,
  pub type_: PokemonTypes,
}

#[diesel_enum(conn = crate::postgres_testing_callback, name_mapping(name = "pokemon_type", path = sql_types::PokemonType))]
pub enum PokemonTypes {
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
