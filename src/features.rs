pub fn default_skip_check() -> bool {
  cfg!(feature = "default-skip-check")
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
