use std::ops::Range;

use convert_case::{Case, Casing};
use quote::{format_ident, quote, ToTokens};
use syn::{
  parse::Parse, punctuated::Punctuated, Error, Expr, Ident, Lit, Meta, Path, RangeLimits, Token,
};

use crate::{
  features::{
    default_conn_function_path, default_name_mapping, default_skip_consistency_check,
    default_skip_test, default_sqlite_runner, no_default_id_mapping,
  },
  Check, TokenStream2,
};

pub struct Attributes<'a> {
  pub table_path: Option<TokenStream2>,
  pub table_name: Option<String>,
  pub column: Option<String>,
  pub conn: Check,
  pub skip_test: bool,
  pub case: Case<'a>,
  pub name_mapping: Option<NameMapping>,
  pub id_mapping: Option<IdMapping>,
  pub skip_ranges: Vec<Range<i32>>,
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

enum IdMappingOrSkip {
  IdMapping(IdMapping),
  Skip,
}

impl Default for IdMappingOrSkip {
  fn default() -> Self {
    Self::IdMapping(IdMapping::default())
  }
}

struct SkippedRanges {
  pub ranges: Vec<Range<i32>>,
}

impl Parse for SkippedRanges {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut ranges: Vec<Range<i32>> = Vec::new();

    let items = Punctuated::<Expr, Token![,]>::parse_terminated(input)?;

    for item in items {
      if let Expr::Range(range_expr) = &item {
        let start = if let Some(start_expr) = &range_expr.start {
          extract_i32(&start_expr)?
        } else {
          0
        };

        let end = if let Some(end_expr) = &range_expr.end {
          extract_i32(&end_expr)?
        } else {
          return Err(spanned_error!(
            range_expr,
            "Infinite range is not supported"
          ));
        };

        let final_end = if let RangeLimits::HalfOpen(_) = &range_expr.limits {
          end
        } else {
          end + 1
        };

        ranges.push(start..final_end);
      } else {
        return Err(spanned_error!(
          item,
          "Expected a range (e.g. `1..5`, `10..=15`)"
        ));
      }
    }

    ranges.sort_by_key(|range| range.start);

    Ok(Self { ranges })
  }
}

impl Parse for IdMappingOrSkip {
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
      } else if ident == "skip" {
        if args_len != 1 {
          return Err(error!(
            input.span(),
            "Cannot use other `id_mapping` attributes when using `skip`"
          ));
        } else {
          return Ok(Self::Skip);
        }
      } else if ident == "sql_type" {
        check_duplicate!(ident, rust_type, "sql_type");

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
          format!("Unknown attribute `{ident}`. Expected one of: `skip`, `default`, `sql_type`")
        ));
      }
    }

    Ok(Self::IdMapping(IdMapping {
      type_path: int_type_path.unwrap_or_else(|| quote! { diesel::sql_types::Integer }),
      rust_type: rust_type.unwrap_or_else(|| format_ident!("i32")),
    }))
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

enum NameMappingOrSkip {
  NameMapping(NameMapping),
  Skip,
}

impl Default for NameMappingOrSkip {
  fn default() -> Self {
    Self::NameMapping(NameMapping::default())
  }
}

impl Parse for NameMappingOrSkip {
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
      } else if ident == "skip" {
        if args_len != 1 {
          return Err(error!(
            input.span(),
            "Cannot use other `name_mapping` attributes when using `skip`"
          ));
        } else {
          return Ok(Self::Skip);
        }
      } else if ident == "path" {
        check_duplicate!(ident, custom_type_path, "path");

        let type_path = extract_path(arg.require_name_value()?.clone().value)?;

        custom_type_path = Some(type_path);
      } else if ident == "name" {
        check_duplicate!(ident, custom_enum_name, "name");

        let db_enum_name = extract_string_lit(&arg.require_name_value()?.value)?;

        custom_enum_name = Some(db_enum_name);
      } else {
        return Err(spanned_error!(
          ident,
          format!(
            "Unknown attribute `{ident}`. Expected one of: `skip`, `default`, `path`, `name`"
          )
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

    Ok(Self::NameMapping(NameMapping {
      db_type,
      path: custom_type_path.map_or_else(
        || quote! { diesel::sql_types::Text },
        |t| t.to_token_stream(),
      ),
    }))
  }
}

pub fn extract_string_lit(expr: &Expr) -> Result<String, Error> {
  if let Expr::Lit(expr_lit) = expr && let Lit::Str(value) = &expr_lit.lit {
    Ok(value.value())
  } else {
    Err(spanned_error!(expr, "Expected a string literal"))
  }
}

pub fn extract_i32(expr: &Expr) -> Result<i32, Error> {
  if let Expr::Lit(expr_lit) = expr && let Lit::Int(value) = &expr_lit.lit {
    Ok(value.base10_parse()?)
  } else {
    Err(spanned_error!(expr, "Expected an integer literal"))
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
    let mut table_name: Option<String> = None;
    let mut table_path: Option<Path> = None;
    let mut column: Option<String> = None;
    let mut conn: Option<Check> = None;
    let mut case: Option<Case> = None;
    let mut name_mapping: Option<NameMappingOrSkip> = None;
    let mut id_mapping: Option<IdMappingOrSkip> = None;
    let mut skip_test: Option<bool> = None;
    let mut force_run_test: Option<bool> = None;
    let mut skip_ids: Option<Vec<Range<i32>>> = None;

    let punctuated_args = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

    let attributes_error_msg =
      "Expected one of: `table_name`, `table`, `column`, `conn`, `skip_consistency_check`, `skip_ids`, `run_test`, `skip_test`, `case`, `id_mapping`, `name_mapping`";

    for arg in punctuated_args {
      match arg {
        Meta::List(list) => {
          let ident = list.path.require_ident()?;

          if ident == "skip_ids" {
            check_duplicate!(ident, skip_ids);

            let ranges = list.parse_args::<SkippedRanges>()?;

            skip_ids = Some(ranges.ranges);
          } else if ident == "name_mapping" {
            check_duplicate!(ident, name_mapping);

            let parse_result = syn::parse2::<NameMappingOrSkip>(list.tokens)?;

            name_mapping = Some(parse_result);
          } else if ident == "id_mapping" {
            check_duplicate!(ident, id_mapping);

            let parse_result = syn::parse2::<IdMappingOrSkip>(list.tokens)?;

            id_mapping = Some(parse_result);
          } else {
            return Err(spanned_error!(
              ident,
              format!("Unknown attribute `{ident}`. {attributes_error_msg}")
            ));
          }
        }
        Meta::Path(path) => {
          let ident = path.require_ident()?;

          if ident == "run_test" {
            check_duplicate!(ident, force_run_test, "run_test");

            if skip_test.is_some() {
              return Err(spanned_error!(
                ident,
                "Cannot use `run_test` and `skip_test` together"
              ));
            }

            force_run_test = Some(true);
          } else if ident == "skip_consistency_check" {
            check_duplicate!(ident, conn, "skip_consistency_check");

            if matches!(conn, Some(Check::Conn(_))) {
              return Err(spanned_error!(
                ident,
                "Cannot use `conn` with `skip_consistency_check`"
              ));
            }

            conn = Some(Check::Skip);
          } else if ident == "skip_test" {
            check_duplicate!(ident, skip_test);

            if force_run_test.is_some() {
              return Err(spanned_error!(
                ident,
                "Cannot use `run_test` and `skip_test` together"
              ));
            }

            skip_test = Some(true);
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

          if ident == "table" {
            check_duplicate!(ident, table_path, "table");

            table_path = Some(extract_path(value)?);
          } else if ident == "case" {
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
          } else if ident == "table_name" {
            check_duplicate!(ident, table_name);

            table_name = Some(extract_string_lit(&value)?);
          } else if ident == "column" {
            check_duplicate!(ident, column);

            column = Some(extract_string_lit(&value)?);
          } else if ident == "conn" {
            check_duplicate!(ident, conn);

            if matches!(conn, Some(Check::Skip)) {
              return Err(spanned_error!(
                ident,
                "Cannot use `conn` with `skip_consistency_check`"
              ));
            }

            conn = Some(Check::Conn(extract_path(value)?.to_token_stream()));
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
    } else if default_skip_consistency_check() {
      Check::Skip
    } else if default_sqlite_runner() {
      Check::Conn(quote! { diesel_enums::test_runners::sqlite_runner })
    } else if default_conn_function_path() {
      Check::Conn(quote! { crate::db_enum_test_func::get_connection })
    } else {
      return Err(error!(
        input.span(),
        "At least one between `conn` and `skip_consistency_check` must be present"
      ));
    };

    let mut is_custom_type = false;

    let name_mapping = if let Some(mapping) = name_mapping {
      match mapping {
        NameMappingOrSkip::NameMapping(mapping) => {
          is_custom_type = mapping.db_type.is_custom();
          Some(mapping)
        }

        NameMappingOrSkip::Skip => None,
      }
    } else if default_name_mapping() {
      Some(NameMapping::default())
    } else {
      None
    };

    let id_mapping = if is_custom_type {
      None
    } else if let Some(mapping) = id_mapping {
      match mapping {
        IdMappingOrSkip::IdMapping(mapping) => Some(mapping),
        IdMappingOrSkip::Skip => None,
      }
    } else if !no_default_id_mapping() {
      Some(IdMapping::default())
    } else {
      None
    };

    if table_name.is_none() && let Some(path) = &table_path {
      let name = &path.segments.last().ok_or(spanned_error!(path.clone(), "Invalid table path"))?.ident;

      table_name = Some(name.to_string());
    }

    let table_path = table_path.map(|path| path.to_token_stream());

    let skip_test =
      force_run_test.unwrap_or_else(|| skip_test.unwrap_or_else(|| default_skip_test()));

    Ok(Attributes {
      table_name,
      table_path,
      column,
      conn,
      case: case.unwrap_or(Case::Snake),
      id_mapping,
      name_mapping,
      skip_test,
      skip_ranges: skip_ids.unwrap_or_default(),
    })
  }
}
