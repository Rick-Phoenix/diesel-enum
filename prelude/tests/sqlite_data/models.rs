use diesel::prelude::*;
use diesel_enums::diesel_enum;

use super::schema::*;

#[derive(Queryable, Selectable, Debug, Identifiable, Insertable)]
pub struct Pokemon {
  pub id: i32,
  pub name: String,
}

// Lookup table used for many-to-many relationship between pokemons and types
#[derive(Queryable, Associations, Insertable)]
#[diesel(belongs_to(Pokemon))]
#[diesel(belongs_to(Type))]
#[diesel(primary_key(pokemon_id, type_id))]
#[diesel(table_name = pokemon_types)]
pub struct PokemonType {
  pub pokemon_id: i32,
  pub type_id: TypesId, // Automatically generated from `Types` since it is a double mapping
}

// We use the enum to reference known, existing types
#[diesel_enum(conn = diesel_enums::sqlite_runner, table = types, name_mapping(default), case = "PascalCase", id_mapping(default))]
pub enum Types {
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

// We keep the generic lookup table structure to insert new types in the future
#[derive(Queryable, Selectable, Debug, Insertable, Identifiable)]
#[diesel(table_name = types)]
pub struct Type {
  #[diesel(skip_insertion)]
  pub id: i32,
  pub name: String,
}
