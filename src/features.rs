pub fn default_skip_check() -> bool {
  cfg!(feature = "default-skip-check")
}

pub fn default_text_impl() -> bool {
  cfg!(feature = "default-text-impl")
}
