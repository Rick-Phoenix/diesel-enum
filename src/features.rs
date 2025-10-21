pub fn default_skip_check() -> bool {
  cfg!(feature = "default-skip-check")
}

pub fn default_text_impl() -> bool {
  cfg!(feature = "default-text-impl")
}

pub fn no_default_int_impl() -> bool {
  cfg!(feature = "no-default-int-impl")
}
