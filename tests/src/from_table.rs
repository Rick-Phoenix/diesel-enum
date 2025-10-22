use diesel_enum_checked::diesel_enum;

#[derive(Clone)]
#[diesel_enum(conn = crate::sqlite_testing_callback, name_mapping(default), case = "PascalCase")]
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
async fn you_shall_pass() {
  TypesId::check_consistency().await;
}
