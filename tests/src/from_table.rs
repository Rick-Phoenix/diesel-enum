use diesel_enum_checked::diesel_enum;

#[tokio::test]
async fn you_shall_pass() {
  crate::models::TypesId::check_consistency().await;
}

mod altered_casing {
  use super::*;

  #[diesel_enum(conn = crate::sqlite_testing_callback, case = "PascalCase", name_mapping(default), id_mapping(default))]
  #[allow(non_camel_case_types)]
  enum Types {
    grass,
    poison,
    fire,
    flying,
    water,
    bug,
    normal,
    electric,
    ground,
    fairy,
    fighting,
    psychic,
    rock,
    steel,
    ice,
    ghost,
    dragon,
    dark,
  }

  #[tokio::test]
  async fn altered_casing() {
    Types::check_consistency().await;
  }
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

  #[diesel_enum(conn = crate::sqlite_testing_callback, case = "PascalCase", name_mapping(default), id_mapping(skip))]
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

  #[diesel_enum(conn = crate::sqlite_testing_callback, case = "PascalCase", name_mapping(default), id_mapping(default))]
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

mod ignored_id_mismatch {
  use super::*;

  #[diesel_enum(conn = crate::sqlite_testing_callback, case = "PascalCase", name_mapping(default), id_mapping(skip))]
  enum Types {
    // Wrong order here
    Poison,
    Grass,
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
  async fn ignored_id_mismatch() {
    Types::check_consistency().await;
  }
}

mod skipped_ids {
  use super::*;

  #[diesel_enum(conn = crate::sqlite_testing_callback, skip_ids(1..6, 6..=10), case = "PascalCase", name_mapping(default), id_mapping(default))]
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
  async fn skipped_ids() {
    Types::check_consistency().await;
  }
}

mod custom_table_name {
  use super::*;

  #[diesel_enum(conn = crate::sqlite_testing_callback, case = "PascalCase", table_name = "types", name_mapping(default), id_mapping(default))]
  enum PokeTypes {
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
  async fn custom_table_name() {
    PokeTypes::check_consistency().await;
  }
}
