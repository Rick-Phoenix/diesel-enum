use convert_case::{Case, Casing};
use quote::{format_ident, quote, ToTokens};
use syn::{parse::Parse, punctuated::Punctuated, Error, Expr, Ident, Lit, Meta, Path, Token};

use crate::{
  features::{default_skip_check, default_text_impl, no_default_int_impl},
  Check, TokenStream2,
};

pub struct Attributes<'a> {
  pub table: Option<String>,
  pub column: Option<String>,
  pub conn: Check,
  pub case: Case<'a>,
  pub name_mapping: Option<NameMapping>,
  pub id_mapping: Option<IdMapping>,
}

pub struct IdMapping {
  pub type_path: TokenStream2,
  pub rust_type: Ident,
}

impl Default for IdMapping {
  fn default() -> Self {
    Self {
      type_path: quote! { diesel::sql_types::Integer },
      rust_type: format_ident!("i32"),
    }
  }
}

impl Parse for IdMapping {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut rust_type: Option<Ident> = None;
    let mut int_type_path: Option<TokenStream2> = None;

    let punctuated_args = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

    let args_len = punctuated_args.len();

    for arg in punctuated_args {
      let ident = arg.path().require_ident()?;

      if ident == "default" {
        if args_len != 1 {
          return Err(error!(
            input.span(),
            "Cannot use other `id_mapping` attributes when using `default`"
          ));
        } else {
          return Ok(Self::default());
        }
      } else if ident == "type" {
        check_duplicate!(ident, rust_type, "type");

        let value = arg.require_name_value()?.clone().value;

        let type_path = extract_path(value)?;

        let type_ident = &type_path
          .segments
          .last()
          .ok_or_else(|| spanned_error!(type_path.clone(), "Invalid type path"))?
          .ident;

        let type_target = if type_ident == "Integer" {
          "i32"
        } else if type_ident == "BigInt" {
          "i64"
        } else if type_ident == "SmallInt" {
          "i16"
        } else if type_ident == "TinyInt" {
          "i8"
        } else {
          return Err(spanned_error!(
            type_ident,
            format!("Unknown ID type {type_ident}. Only valid integer types from `diesel::sql_types` are accepted")));
        };

        rust_type = Some(format_ident!("{type_target}"));
        int_type_path = Some(type_path.to_token_stream());
      } else {
        return Err(spanned_error!(
          ident,
          format!("Unknown attribute `{ident}`. Expected one of: `default`, `type`")
        ));
      }
    }

    Ok(Self {
      type_path: int_type_path.unwrap_or_else(|| quote! { diesel::sql_types::Integer }),
      rust_type: rust_type.unwrap_or_else(|| format_ident!("i32")),
    })
  }
}

pub enum NameTypes {
  Text,
  Custom { name: String },
}

impl NameTypes {
  pub fn is_custom(&self) -> bool {
    matches!(self, Self::Custom { .. })
  }
}

pub struct NameMapping {
  pub db_type: NameTypes,
  pub path: TokenStream2,
}

impl Default for NameMapping {
  fn default() -> Self {
    Self {
      db_type: NameTypes::Text,
      path: quote! { diesel::sql_types::Text },
    }
  }
}

impl Parse for NameMapping {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut custom_type_path: Option<Path> = None;
    let mut custom_enum_name: Option<String> = None;

    let punctuated_args = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

    let args_len = punctuated_args.len();

    for arg in punctuated_args {
      let ident = arg.path().require_ident()?;

      if ident == "default" {
        if args_len != 1 {
          return Err(error!(
            input.span(),
            "Cannot use other `name_mapping` attributes when using `default`"
          ));
        } else {
          return Ok(Self::default());
        }
      } else if ident == "type" {
        check_duplicate!(ident, custom_type_path, "type");

        let type_path = extract_path(arg.require_name_value()?.clone().value)?;

        custom_type_path = Some(type_path);
      } else if ident == "name" {
        check_duplicate!(ident, custom_enum_name, "name");

        let db_enum_name = extract_string_lit(&arg.require_name_value()?.value)?;

        custom_enum_name = Some(db_enum_name);
      } else {
        return Err(spanned_error!(
          ident,
          format!("Unknown attribute `{ident}`. Expected one of: `default`, `type`, `name`")
        ));
      }
    }

    let db_type = if let Some(path) = &custom_type_path {
      let db_name = if let Some(name) = custom_enum_name {
        name
      } else {
        let rust_type_name = path
          .segments
          .last()
          .ok_or_else(|| spanned_error!(path.clone(), "Invalid path attribute"))?;

        // Falling back to snake cased name of custom type struct
        rust_type_name.ident.to_string().to_case(Case::Snake)
      };

      NameTypes::Custom { name: db_name }
    } else {
      NameTypes::Text
    };

    Ok(Self {
      db_type,
      path: custom_type_path.map_or_else(
        || quote! { diesel::sql_types::Text },
        |t| t.to_token_stream(),
      ),
    })
  }
}

pub fn extract_string_lit(expr: &Expr) -> Result<String, Error> {
  if let Expr::Lit(expr_lit) = expr && let Lit::Str(value) = &expr_lit.lit {
    Ok(value.value())
  } else {
    Err(spanned_error!(expr, "Expected a string literal"))
  }
}

pub fn extract_path(expr: Expr) -> Result<Path, Error> {
  if let Expr::Path(expr_path) = expr {
    Ok(expr_path.path)
  } else {
    Err(spanned_error!(expr, "Expected a Path"))
  }
}

impl<'a> Parse for Attributes<'a> {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut table: Option<String> = None;
    let mut column: Option<String> = None;
    let mut conn: Option<Check> = None;
    let mut case: Option<Case> = None;
    let mut name_mapping: Option<NameMapping> = None;
    let mut id_mapping: Option<IdMapping> = None;

    let punctuated_args = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

    let attributes_error_msg =
      "Expected one of: `table`, `column`, `conn`, `skip_check`, `case`, `id_mapping`, `name_mapping`";

    for arg in punctuated_args {
      match arg {
        Meta::List(list) => {
          let ident = list.path.require_ident()?;

          if ident == "name_mapping" {
            check_duplicate!(ident, name_mapping);

            name_mapping = Some(syn::parse2::<NameMapping>(list.tokens)?);
          } else if ident == "id_mapping" {
            check_duplicate!(ident, id_mapping);

            id_mapping = Some(syn::parse2::<IdMapping>(list.tokens)?);
          } else {
            return Err(spanned_error!(
              ident,
              format!("Unknown attribute `{ident}`. {attributes_error_msg}")
            ));
          }
        }
        Meta::Path(path) => {
          let ident = path.require_ident()?;

          if ident == "skip_check" {
            check_duplicate!(ident, conn, "skip_check");

            if matches!(conn, Some(Check::Conn(_))) {
              return Err(spanned_error!(ident, "Cannot use `conn` with `skip_check`"));
            }

            conn = Some(Check::Skip);
          } else {
            return Err(spanned_error!(
              ident,
              format!("Unknown attribute `{ident}`. {attributes_error_msg}")
            ));
          }
        }
        Meta::NameValue(arg) => {
          let ident = arg.path.require_ident()?;
          let value = arg.value;

          if ident == "case" {
            check_duplicate!(ident, case);

            let case_value = match extract_string_lit(&value)?.as_str()  {
              "snake_case" => Case::Snake,
              "UPPER_SNAKE" => Case::UpperSnake,
              "camelCase" => Case::Camel,
              "PascalCase" => Case::Pascal,
              "lowercase" => Case::Lower,
              "UPPERCASE" => Case::Upper,
              "kebab-case" => Case::Kebab,
              _ => return Err(spanned_error!(value, "Invalid value for `case`. Allowed values are: [ snake_case, UPPER_SNAKE, camelCase, PascalCase, lowercase, UPPERCASE, kebab-case ]"))
            };

            case = Some(case_value);
          } else if ident == "table" {
            check_duplicate!(ident, table);

            table = Some(extract_string_lit(&value)?);
          } else if ident == "column" {
            check_duplicate!(ident, column);

            column = Some(extract_string_lit(&value)?);
          } else if ident == "conn" {
            check_duplicate!(ident, conn);

            if matches!(conn, Some(Check::Skip)) {
              return Err(spanned_error!(ident, "Cannot use `conn` with `skip_check`"));
            }

            conn = Some(Check::Conn(extract_path(value)?));
          } else {
            return Err(spanned_error!(
              ident,
              format!("Unknown attribute `{ident}`, {attributes_error_msg}")
            ));
          }
        }
      };
    }

    let conn = if let Some(input) = conn {
      input
    } else if default_skip_check() {
      Check::Skip
    } else {
      return Err(error!(
        input.span(),
        "At least one between `conn` and `skip_check` must be present"
      ));
    };

    if id_mapping.is_none() && !no_default_int_impl() {
      id_mapping = Some(IdMapping::default());
    }

    if name_mapping.is_none() && default_text_impl() {
      name_mapping = Some(NameMapping::default());
    }

    Ok(Attributes {
      table,
      column,
      conn,
      case: case.unwrap_or(Case::Snake),
      id_mapping,
      name_mapping,
    })
  }
}
