pub fn default_skip_check() -> bool {
  cfg!(feature = "default-skip-check")
}

pub fn default_auto_increment() -> bool {
  cfg!(feature = "default-auto-increment")
}

pub fn default_map_int() -> bool {
  cfg!(feature = "default-map-int")
}
