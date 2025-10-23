use diesel::prelude::*;
use diesel_enums::diesel_enum;

use crate::{pg_schema, schema::*};

#[derive(Queryable, Selectable, Debug, Identifiable, Insertable)]
pub struct Pokemon {
  pub id: i32,
  pub name: String,
  pub next_evolution_id: Option<i32>,
  pub prev_evolution_id: Option<i32>,
  pub description: String,
  pub image_data_id: i32,
  pub base_stats_id: i32,
}

#[derive(Queryable, Identifiable, Associations, Insertable, Selectable, Debug)]
#[diesel(belongs_to(Pokemon))]
pub struct BaseStat {
  #[diesel(skip_insertion)]
  pub id: i32,
  pub hp: i32,
  pub attack: i32,
  pub defense: i32,
  pub special_attack: i32,
  pub special_defense: i32,
  pub speed: i32,
  pub pokemon_id: i32,
}

#[derive(Queryable, Identifiable, Associations, Insertable, Selectable, Debug)]
#[diesel(table_name = image_data)]
#[diesel(belongs_to(Pokemon))]
pub struct ImageData {
  #[diesel(skip_insertion)]
  pub id: i32,
  pub sprite: String,
  pub thumbnail: String,
  pub hires: String,
  pub pokemon_id: i32,
}

#[derive(Queryable, Associations, Insertable)]
#[diesel(belongs_to(Pokemon))]
#[diesel(belongs_to(Type))]
#[diesel(primary_key(pokemon_id, type_id))]
#[diesel(table_name = pokemon_types)]
pub struct PokemonType {
  pub pokemon_id: i32,
  pub type_id: TypesId,
}

#[diesel_enum(conn = crate::sqlite_testing_callback, name_mapping(default), case = "PascalCase", id_mapping(default))]
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

#[derive(Queryable, Selectable, Debug, Insertable, Identifiable)]
#[diesel(table_name = types)]
pub struct Type {
  #[diesel(skip_insertion)]
  pub id: i32,
  pub name: String,
}

#[derive(Queryable, Selectable, Debug, Insertable, PartialEq, Clone)]
#[diesel(table_name = pg_schema::pokemon_table)]
pub struct PgTable {
  pub name: String,
  pub type_: PgTypes,
}

#[diesel_enum(conn = crate::postgres_testing_callback, name_mapping(name = "pokemon_type", path = pg_schema::sql_types::PokemonType))]
pub enum PgTypes {
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
