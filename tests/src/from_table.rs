use diesel_enum_checked::diesel_enum;

#[tokio::test]
async fn you_shall_pass() {
  crate::models::TypesId::check_consistency().await;
}

mod wrong_casing {
  use super::*;

  #[diesel_enum(conn = crate::sqlite_testing_callback, name_mapping(default), id_mapping(skip))]
  enum Types {
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
    Types::check_consistency().await;
  }
}

mod name_mismatch {
  use super::*;

  #[diesel_enum(conn = crate::sqlite_testing_callback, name_mapping(default), id_mapping(skip))]
  enum Types {
    #[db_mapping(name = "abc")]
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
  async fn name_mismatch() {
    Types::check_consistency().await;
  }
}

mod id_mismatch {
  use super::*;

  #[diesel_enum(conn = crate::sqlite_testing_callback, name_mapping(default), id_mapping(default))]
  enum Types {
    #[db_mapping(id = 20)]
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
  async fn id_mismatch() {
    Types::check_consistency().await;
  }
}
