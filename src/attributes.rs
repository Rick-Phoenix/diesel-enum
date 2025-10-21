use convert_case::Case;
use quote::{quote, ToTokens};
use syn::{parse::Parse, punctuated::Punctuated, Error, Expr, Lit, Meta, Path, Token};

use crate::{
  features::{default_auto_increment, default_map_int, default_skip_check},
  Check, MappedType, SqlType,
};

pub struct Attributes<'a> {
  pub table: Option<String>,
  pub column: Option<String>,
  pub name: Option<String>,
  pub conn: Check,
  pub case: Option<Case<'a>>,
  pub sql_type: SqlType,
  pub auto_increment: bool,
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
    let mut sql_type: Option<SqlType> = None;
    let mut name: Option<String> = None;
    let mut auto_increment: Option<bool> = None;

    let punctuated_args = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

    let attributes_error_msg =
      "expected one of: <`name` | `table`, `column`>, `conn`, `skip_check`, `case`, `sql_type`, `auto_increment`";

    for arg in punctuated_args {
      match arg {
        Meta::Path(path) => {
          let ident = path.require_ident()?;

          if ident == "skip_check" {
            check_duplicate!(ident, conn, "skip_check");

            if matches!(conn, Some(Check::Conn(_))) {
              return Err(spanned_error!(ident, "Cannot use `conn` with `skip_check`"));
            }

            conn = Some(Check::Skip);
          } else if ident == "auto_increment" {
            check_duplicate!(ident, auto_increment);

            auto_increment = Some(true);
          } else {
            return Err(spanned_error!(
              ident,
              format!("Unknown attribute `{ident}`, {attributes_error_msg}")
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
          } else if ident == "name" {
            check_duplicate!(ident, name);

            name = Some(extract_string_lit(&value)?);
          } else if ident == "sql_type" {
            check_duplicate!(ident, sql_type);

            let type_path = extract_path(value)?;

            let type_ident = &type_path
              .segments
              .last()
              .expect("Missing path segments")
              .ident;

            let type_target = if type_ident == "Text" {
              MappedType::Text
            } else if type_ident == "Integer" {
              MappedType::I32
            } else if type_ident == "BigInt" {
              MappedType::I64
            } else if type_ident == "SmallInt" {
              MappedType::I16
            } else if type_ident == "TinyInt" {
              MappedType::I8
            } else {
              MappedType::Custom
            };

            sql_type = Some(SqlType {
              db_type: type_target,
              path: type_path.to_token_stream(),
            });
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
        Meta::List(list) => {
          return Err(spanned_error!(list, "Expected a path or key-value pair"));
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

    let sql_type = if let Some(sql_type) = sql_type {
      sql_type
    } else if default_map_int() {
      SqlType {
        db_type: MappedType::I32,
        path: quote! { diesel::sql_types::Integer },
      }
    } else {
      return Err(error!(input.span(), "No `sql_type` has been set"));
    };

    if let Some(true) = auto_increment && matches!(sql_type.db_type, MappedType::Text | MappedType::Custom) {
      return Err(error!(
        input.span(),
        "Cannot use `auto_increment` when the mapped type is not integer-based"
      ));
    }

    if name.is_some() && (table.is_some() || column.is_some()) {
      return Err(error!(
        input.span(),
        "`name` cannot be used with `table` or `column`"
      ));
    }

    // Only falling back here, in case someone is using the default but overriding it
    let auto_increment = auto_increment.unwrap_or_else(|| default_auto_increment());

    Ok(Attributes {
      table,
      column,
      conn,
      case,
      sql_type,
      auto_increment,
      name,
    })
  }
}
