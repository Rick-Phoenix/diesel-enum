use std::borrow::Borrow;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
pub(crate) use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse::Parse, parse_macro_input, Error, Expr, Ident, ItemEnum, Lit, Meta, Path, Token};

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

enum Check {
  Conn(Path),
  Skip,
}

struct Attributes<'a> {
  pub table: Option<String>,
  pub column: Option<String>,
  pub conn: Check,
  pub case: Option<Case<'a>>,
}

fn extract_string_lit(expr: &Expr) -> Result<String, Error> {
  if let Expr::Lit(expr_lit) = expr && let Lit::Str(value) = &expr_lit.lit {
    Ok(value.value())
  } else {
    Err(spanned_error!(expr, "Expected a string literal"))
  }
}

fn extract_path(expr: Expr) -> Result<Path, Error> {
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

    let punctuated_args = syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

    for arg in punctuated_args {
      match arg {
        Meta::Path(path) => {
          let ident = path.require_ident()?;

          if ident == "skip_check" {
            conn = Some(Check::Skip);
          } else {
            return Err(spanned_error!(
              ident,
              format!(
                "Unknown attribute `{}`, expected one of: `table`, `column`, `conn`, `skip_check` or `case`",
                ident
              )
            ));
          }
        }
        Meta::NameValue(arg) => {
          let ident = arg.path.require_ident()?;
          let value = arg.value;

          if ident == "case" {
            if case.is_some() {
              return Err(spanned_error!(ident, "Duplicate attribute `case`"));
            }

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
            if table.is_some() {
              return Err(spanned_error!(ident, "Duplicate attribute `table`"));
            }

            table = Some(extract_string_lit(&value)?);
          } else if ident == "column" {
            if column.is_some() {
              return Err(spanned_error!(ident, "Duplicate attribute `column`"));
            }

            column = Some(extract_string_lit(&value)?);
          } else if ident == "conn" {
            if conn.is_some() {
              return Err(spanned_error!(ident, "Duplicate attribute `conn`"));
            }

            conn = Some(Check::Conn(extract_path(value)?));
          } else {
            return Err(spanned_error!(
              ident,
              format!(
                "Unknown attribute `{}`, expected one of: `table`, `column`, `conn`, `skip_check` or `case`",
                ident
              )
            ));
          }
        }
        Meta::List(list) => {
          return Err(spanned_error!(list, "Expected a path or key-value pair"));
        }
      };
    }

    let conn = conn.ok_or_else(|| {
      error!(
        input.span(),
        "At least one between `conn` and `skip_check` must be present"
      )
    })?;

    Ok(Attributes {
      table,
      column,
      conn,
      case,
    })
  }
}

fn traverse_enum<T, V>(variants: &[V], action: T) -> TokenStream2
where
  T: Fn(&Ident) -> TokenStream2,
  V: Borrow<Ident>,
{
  let mut tokens = TokenStream2::new();

  for variant in variants {
    let variant = variant.borrow();
    let action_tokens = action(variant);
    tokens.extend(quote! {
      #action_tokens,
    });
  }

  tokens
}

#[proc_macro_attribute]
pub fn diesel_sqlite_enum(attrs: TokenStream, input: TokenStream) -> TokenStream {
  let orig_input: TokenStream2 = input.clone().into();

  let attributes = parse_macro_input!(attrs as Attributes);

  let Attributes {
    table: table_name,
    column: column_name,
    conn: check,
    case: db_variants_case,
  } = attributes;

  let ast = parse_macro_input!(input as ItemEnum);

  let variant_idents: Vec<&Ident> = ast.variants.iter().map(|v| &v.ident).collect();

  let variant_db_names: Vec<String> = ast
    .variants
    .iter()
    .map(|v| {
      if let Some(case) = db_variants_case {
        v.ident.to_string().to_case(case)
      } else {
        v.ident.to_string()
      }
    })
    .collect();

  let enum_name = &ast.ident;
  let enum_name_str = enum_name.to_string();

  let display_impl_body = if db_variants_case.is_none() {
    traverse_enum(&variant_idents, |variant| {
      quote! {
        Self::#variant => stringify!(#variant).to_string()
      }
    })
  } else {
    let mut tokens = TokenStream2::new();

    variant_idents
      .iter()
      .zip(variant_db_names.iter())
      .for_each(|(variant, db_name)| {
        tokens.extend(quote! {
          Self::#variant => #db_name,
        });
      });

    tokens
  };

  let try_from_str_body = if db_variants_case.is_none() {
    traverse_enum(&variant_idents, |variant| {
      quote! {
        stringify!(#variant) => Ok(Self::#variant)
      }
    })
  } else {
    let mut tokens = TokenStream2::new();

    variant_idents
      .iter()
      .zip(variant_db_names.iter())
      .for_each(|(variant, db_name)| {
        tokens.extend(quote! {
          #db_name => Ok(Self::#variant),
        });
      });

    tokens
  };

  let test_impl = match check {
    Check::Conn(connection_func) => {
      let table_name = table_name.unwrap_or_else(|| enum_name_str.to_case(Case::Snake));
      let table_name_ident = format_ident!("{table_name}");

      let column_name = column_name.unwrap_or_else(|| "name".to_string());
      let column_name_ident = format_ident!("{column_name}");

      let test_mod_name =
        format_ident!("__diesel_enum_test_{}", enum_name_str.to_case(Case::Snake));

      let test_func_name = format_ident!("diesel_enum_test_{}", enum_name_str.to_case(Case::Snake));

      Some(quote! {
        #[cfg(test)]
        mod #test_mod_name {
          use super::*;
          use diesel::prelude::*;
          use std::collections::HashSet;
          use crate::schema::#table_name_ident;
          use std::fmt::Write;

          #[test]
          fn #test_func_name() {
            let mut rust_variants = HashSet::from({
              [ #(#variant_db_names),* ]
            });

            let conn = &mut #connection_func();

            let db_variants: Vec<String> = #table_name_ident::table
              .select(#table_name_ident::#column_name_ident)
              .load(conn)
              .unwrap_or_else(|_| panic!("Failed to load variants from the database for the enum `{}`.", #enum_name_str));

            let mut missing_variants: Vec<String> = Vec::new();

            for variant in db_variants {
              let was_present = rust_variants.remove(variant.as_str());

              if !was_present {
                missing_variants.push(variant);
              }
            }

            if !missing_variants.is_empty() || !rust_variants.is_empty() {
              let mut error_message = String::new();

              write!(error_message, "The rust enum `{}` and the database table `{}` are out of sync: ", #enum_name_str, #table_name).unwrap();

              if !missing_variants.is_empty() {
                missing_variants.sort();

                write!(error_message, "\n  - Variants missing from the rust enum: [ {} ]", missing_variants.join(", ")).unwrap();
              }

              if !rust_variants.is_empty() {
                let mut excess_variants: Vec<&str> = rust_variants.into_iter().collect();
                excess_variants.sort();

                write!(error_message, "\n  - Variants missing from DB: [ {} ]", excess_variants.join(", ")).unwrap();
              }

              panic!("{error_message}");
            }
          }
        }
      })
    }
    Check::Skip => None,
  };

  let output = quote! {
    impl diesel::deserialize::FromSql<diesel::sql_types::Text, diesel::sqlite::Sqlite> for #enum_name {
      fn from_sql(bytes: diesel::sqlite::SqliteValue) -> diesel::deserialize::Result<Self> {
        let value = <String as diesel::deserialize::FromSql<diesel::sql_types::Text, diesel::sqlite::Sqlite>>::from_sql(bytes)?;

        match value.as_str() {
          #try_from_str_body
          _ => Err(Box::from(format!("Unknown {}: {}", stringify!(#enum_name), value))),
        }
      }
    }

    impl diesel::serialize::ToSql<diesel::sql_types::Text, diesel::sqlite::Sqlite> for #enum_name {
      fn to_sql<'b>(&'b self, out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>) -> diesel::serialize::Result {
        let value = match self {
          #display_impl_body
        };

        out.set_value(value);
        Ok(diesel::serialize::IsNull::No)
      }
    }

    #test_impl

    #[derive(diesel::deserialize::FromSqlRow, diesel::expression::AsExpression)]
    #[diesel(sql_type = diesel::sql_types::Text)]
    #orig_input
  };

  output.into()
}
