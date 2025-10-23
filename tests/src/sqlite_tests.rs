use diesel_enums::{diesel_enum, ErrorKind};

#[tokio::test]
async fn you_shall_pass() {
  crate::models::TypesId::check_consistency().await.unwrap();
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
    Types::check_consistency().await.unwrap();
  }
}

mod wrong_casing {
  use super::*;

  #[diesel_enum(conn = crate::sqlite_testing_callback, skip_test, name_mapping(default), id_mapping(skip))]
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
  async fn wrong_casing() {
    let errors = Types::check_consistency().await.unwrap_err().errors;

    assert_eq!(errors.len(), 2);

    assert!(errors.iter().any(|e| {
      if let ErrorKind::MissingFromDb(items) = e {
        items.len() == 18
      } else if let ErrorKind::MissingFromRustEnum(items) = e {
        items.len() == 18
      } else {
        false
      }
    }));
  }
}

mod name_mismatch {
  use super::*;

  #[diesel_enum(conn = crate::sqlite_testing_callback, skip_test, case = "PascalCase", name_mapping(default), id_mapping(skip))]
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
  async fn name_mismatch() {
    let errors = Types::check_consistency().await.unwrap_err().errors;

    assert_eq!(errors.len(), 2);

    assert!(errors.iter().any(|e| {
      if let ErrorKind::MissingFromDb(items) = e {
        assert_eq!(items.len(), 1);
        assert_eq!(items[0], "abc");
        true
      } else if let ErrorKind::MissingFromRustEnum(items) = e {
        assert_eq!(items.len(), 1);
        assert_eq!(items[0], "Grass");
        true
      } else {
        false
      }
    }));
  }
}

mod id_mismatch {
  use super::*;

  #[diesel_enum(conn = crate::sqlite_testing_callback, skip_test, case = "PascalCase", name_mapping(default), id_mapping(default))]
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
  async fn id_mismatch() {
    let errors = Types::check_consistency().await.unwrap_err().errors;

    assert_eq!(errors.len(), 1);

    let e = errors.first().unwrap();

    if let ErrorKind::IdMismatches(items) = e {
      let (name, expected, found) = items.first().unwrap();

      assert_eq!(name, "Grass");
      assert_eq!(*expected, 1);
      assert_eq!(*found, 20);
    } else {
      panic!();
    }
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
    Types::check_consistency().await.unwrap();
  }
}

mod skipped_ids {
  use super::*;

  #[diesel_enum(conn = crate::sqlite_testing_callback, skip_test, skip_ids(1..6, 6..=10), case = "PascalCase", name_mapping(default), id_mapping(default))]
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
  async fn skipped_ids() {
    let errors = Types::check_consistency().await.unwrap_err().errors;

    assert_eq!(errors.len(), 1);

    let e = errors.first().unwrap();

    if let ErrorKind::IdMismatches(items) = e {
      assert_eq!(items.len(), 18);

      for (i, (_, expected, found)) in items.iter().enumerate() {
        assert_eq!(*expected, (i + 1) as i64);
        assert_eq!(*found, (i + 11) as i64);
      }
    } else {
      panic!();
    }
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
    PokeTypes::check_consistency().await.unwrap();
  }
}
