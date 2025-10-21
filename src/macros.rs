macro_rules! error {
  ($span:expr, $error:expr) => {
    syn::Error::new($span, $error)
  };
}

macro_rules! spanned_error {
  ($expr:expr, $error:expr) => {
    syn::Error::new_spanned($expr, $error)
  };
}

macro_rules! check_duplicate {
  ($ident:ident, $value:ident, $name:literal) => {
    if $value.is_some() {
      return Err(spanned_error!(
        $ident,
        concat!("Duplicate attribute `", $name, "`")
      ));
    }
  };

  ($ident:ident, $value:ident) => {
    if $value.is_some() {
      return Err(spanned_error!(
        $ident,
        concat!("Duplicate attribute `", stringify!($value), "`")
      ));
    }
  };
}
