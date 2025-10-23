pub fn default_skip_consistency_check() -> bool {
  cfg!(feature = "default-skip-consistency-check")
}

pub fn default_skip_test() -> bool {
  cfg!(feature = "default-skip-test")
}

pub fn default_name_mapping() -> bool {
  cfg!(feature = "default-name-mapping")
}

pub fn no_default_id_mapping() -> bool {
  cfg!(feature = "no-default-id-mapping")
}

pub fn default_runner_path() -> bool {
  cfg!(feature = "default-runner-path")
}

pub fn default_sqlite_runner() -> bool {
  cfg!(feature = "default-sqlite-runner")
}

pub fn default_postgres_runner() -> bool {
  cfg!(feature = "default-postgres-runner")
}
