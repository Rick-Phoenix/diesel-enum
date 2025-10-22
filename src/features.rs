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

pub fn async_tests() -> bool {
  cfg!(feature = "async-tests")
}

pub fn pretty_test_errors() -> bool {
  cfg!(feature = "pretty-test-errors")
}

pub fn default_conn_function_path() -> bool {
  cfg!(feature = "default-conn-function-path")
}
