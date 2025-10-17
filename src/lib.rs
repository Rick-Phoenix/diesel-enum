#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Span;
pub(crate) use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
  parse::Parse, parse_macro_input, punctuated::Punctuated, Error, Expr, Ident, ItemEnum, Lit,
  LitInt, LitStr, Meta, Path, Token, Variant,
};

fn default_skip_check() -> bool {
  cfg!(feature = "default-skip-check")
}

fn default_auto_increment() -> bool {
  cfg!(feature = "default-auto-increment")
}

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

enum Check {
  Conn(Path),
  Skip,
}

#[derive(Clone, Copy)]
enum MappedType {
  Text,
  I32,
  I64,
  I16,
}

struct Attributes<'a> {
  pub table: Option<String>,
  pub column: Option<String>,
  pub conn: Check,
  pub case: Option<Case<'a>>,
  pub mapped_type: Option<MappedType>,
  pub auto_increment: bool,
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
    let mut mapped_type: Option<MappedType> = None;
    let mut auto_increment: Option<bool> = None;

    let punctuated_args = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

    let attributes_error_msg =
      "expected one of: `table`, `column`, `conn`, `skip_check`, `case`, `type`, `auto_increment`";

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
          } else if ident == "type" {
            check_duplicate!(ident, mapped_type, "type");

            let type_ident = extract_path(value)?;

            let type_ident = type_ident.require_ident()?;

            let type_target = if type_ident == "text" {
              MappedType::Text
            } else if type_ident == "i32" {
              MappedType::I32
            } else if type_ident == "i64" {
              MappedType::I64
            } else if type_ident == "i16" {
              MappedType::I16
            } else {
              return Err(spanned_error!(
                type_ident,
                "Invalid `type` attribute. Accepted values are: [ text, i16, i32, i64 ]"
              ));
            };

            mapped_type = Some(type_target);
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

    if matches!(mapped_type, Some(MappedType::Text)) && matches!(auto_increment, Some(true)) {
      return Err(error!(
        input.span(),
        "Cannot use `auto_increment` when the mapped type is `Text`"
      ));
    }

    // Only falling back here, in case someone is using the default but overriding it
    let auto_increment = auto_increment.unwrap_or_else(|| default_auto_increment());

    Ok(Attributes {
      table,
      column,
      conn,
      case,
      mapped_type,
      auto_increment,
    })
  }
}

struct VariantData {
  pub ident: Ident,
  pub db_name: String,
  pub id: i64,
  pub skip_check: bool,
}

fn process_variants(
  variants: Punctuated<Variant, Token![,]>,
  case: Option<Case>,
) -> Result<Vec<VariantData>, Error> {
  let mut variants_data: Vec<VariantData> = Vec::new();

  for (i, variant) in variants.iter().enumerate() {
    let ident = variant.ident.clone();
    let mut db_name: Option<String> = None;
    let mut id: Option<i64> = None;
    let mut skip_check = false;

    let i = (i + 1) as i64;

    for attr in &variant.attrs {
      if attr.meta.path().is_ident("diesel_enum") {
        attr.parse_nested_meta(|meta| {
          if meta.path.is_ident("id") {
            let val = meta.value()?;
            id = Some(val.parse::<LitInt>()?.base10_parse::<i64>()?);
          } else if meta.path.is_ident("name") {
            let val = meta.value()?;

            db_name = Some(val.parse::<LitStr>()?.value());
          } else if meta.path.is_ident("skip_check") {
            skip_check = true;
          } else {
            return Err(
              meta.error("Unknown attribute. Allowed attributes are: [ id, name, skip_check ]"),
            );
          }

          Ok(())
        })?
      }
    }

    variants_data.push(VariantData {
      ident,
      db_name: db_name.unwrap_or_else(|| {
        if let Some(case) = case {
          variant.ident.to_string().to_case(case)
        } else {
          variant.ident.to_string()
        }
      }),
      id: id.unwrap_or(i),
      skip_check,
    });
  }

  Ok(variants_data)
}

fn traverse_enum<T>(variants: &[VariantData], action: T) -> TokenStream2
where
  T: Fn(&VariantData) -> TokenStream2,
{
  let mut tokens = TokenStream2::new();

  for variant in variants {
    let action_tokens = action(variant);
    tokens.extend(quote! {
      #action_tokens
    });
  }

  tokens
}

#[proc_macro_attribute]
pub fn diesel_enum(attrs: TokenStream, input: TokenStream) -> TokenStream {
  let orig_input: TokenStream2 = input.clone().into();

  let attributes = parse_macro_input!(attrs as Attributes);

  let mapped_type = attributes.mapped_type;

  let ast = parse_macro_input!(input as ItemEnum);

  let variants_data = match process_variants(ast.variants, attributes.case) {
    Ok(data) => data,
    Err(e) => return e.to_compile_error().into(),
  };

  let enum_name = &ast.ident;
  let enum_name_str = enum_name.to_string();

  let mapped_type = mapped_type.unwrap_or(MappedType::I32);

  match mapped_type {
    MappedType::Text => process_text_enum(
      orig_input,
      enum_name,
      enum_name_str,
      variants_data,
      attributes,
    ),
    _ => process_int_enum(
      orig_input,
      enum_name,
      enum_name_str,
      variants_data,
      attributes,
    ),
  }
}

fn process_int_enum(
  orig_input: TokenStream2,
  enum_name: &Ident,
  enum_name_str: String,
  variants_data: Vec<VariantData>,
  attributes: Attributes,
) -> TokenStream {
  let Attributes {
    table: table_name,
    column: column_name,
    mapped_type,
    conn: check,
    auto_increment,
    ..
  } = attributes;

  let (rust_type, sql_type) = match mapped_type.unwrap_or(MappedType::I32) {
    MappedType::I16 => (format_ident!("i16"), format_ident!("SmallInt")),
    MappedType::I32 => (format_ident!("i32"), format_ident!("Integer")),
    MappedType::I64 => (format_ident!("i64"), format_ident!("BigInt")),
    MappedType::Text => unreachable!(),
  };

  let variants_map = {
    let mut collection_tokens = TokenStream2::new();

    let variants_map_ident = format_ident!("map");

    for variant in &variants_data {
      let db_name = &variant.db_name;
      let variant_ident = &variant.ident;
      collection_tokens.extend(quote! {
        #variants_map_ident.insert(#db_name, #enum_name::#variant_ident.into());
      });
    }

    quote! {
      let mut #variants_map_ident: HashMap<&'static str, #rust_type> = HashMap::new();

      #collection_tokens

      #variants_map_ident
    }
  };

  let int_conversion = if auto_increment {
    let mut into_int = TokenStream2::new();

    let mut from_int = TokenStream2::new();

    for variant in &variants_data {
      let id = LitInt::new(&format!("{}{}", variant.id, rust_type), Span::call_site());
      let variant_ident = &variant.ident;

      into_int.extend(quote! {
        Self::#variant_ident => #id,
      });

      from_int.extend(quote! {
        #id => Ok(Self::#variant_ident),
      });
    }

    Some(quote! {
      impl TryFrom<#rust_type> for #enum_name {
        type Error = String;

        fn try_from(value: #rust_type) -> Result<Self, Self::Error> {
          match value {
            #from_int
            _ => Err(format!("Unknown {}: {}", stringify!(#enum_name), value)),
          }
        }
      }

      impl Into<#rust_type> for #enum_name {
        fn into(self) -> #rust_type {
          match self {
            #into_int
          }
        }
      }
    })
  } else {
    None
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
          use std::collections::HashMap;
          use crate::schema::#table_name_ident;
          use std::fmt::Write;

          #[test]
          fn #test_func_name() {
            let mut rust_variants: HashMap<&'static str, #rust_type> = {
              #variants_map
            };

            let conn = &mut #connection_func();

            let db_variants: Vec<(#rust_type, String)> = #table_name_ident::table
              .select((#table_name_ident::id, #table_name_ident::#column_name_ident))
              .load(conn)
              .unwrap_or_else(|_| panic!("Failed to load variants from the database for the enum `{}`.", #enum_name_str));

            let mut missing_variants: Vec<String> = Vec::new();

            let mut id_mismatches: Vec<(String, #rust_type, #rust_type)> = Vec::new();

            for (id, name) in db_variants {
              let variant_id = if let Some(variant) = rust_variants.remove(name.as_str()) {
                variant
              } else {
                missing_variants.push(name);
                continue;
              };

              if id != variant_id {
                id_mismatches.push((name, id, variant_id));
              }
            }

            if !missing_variants.is_empty() || !rust_variants.is_empty() || !id_mismatches.is_empty() {
              let mut error_message = String::new();

              write!(error_message, "The rust enum `{}` and the database column `{}.{}` are out of sync: ", #enum_name_str, #table_name, #column_name).unwrap();

              for ((name, expected, found)) in id_mismatches {
                write!(error_message, "\n  - Wrong integer conversion for `{name}`. Expected: {expected}, found: {found}").unwrap();
              }

              if !missing_variants.is_empty() {
                missing_variants.sort();

                write!(error_message, "\n  - Variants missing from the rust enum: [ {} ]", missing_variants.join(", ")).unwrap();
              }

              if !rust_variants.is_empty() {
                let mut excess_variants: Vec<&str> = rust_variants.into_iter().map(|(name, _)| name).collect();
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
    #int_conversion

    impl diesel::deserialize::FromSql<diesel::sql_types::#sql_type, diesel::sqlite::Sqlite> for #enum_name {
      fn from_sql(bytes: diesel::sqlite::SqliteValue) -> diesel::deserialize::Result<Self> {
        let value = <#rust_type as diesel::deserialize::FromSql<diesel::sql_types::#sql_type, diesel::sqlite::Sqlite>>::from_sql(bytes)?;

        Ok(value.try_into()?)
      }
    }

    impl diesel::serialize::ToSql<diesel::sql_types::#sql_type, diesel::sqlite::Sqlite> for #enum_name {
      fn to_sql<'b>(&'b self, out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>) -> diesel::serialize::Result {
        let value: #rust_type = self.clone().into();

        out.set_value(value);
        Ok(diesel::serialize::IsNull::No)
      }
    }

    #test_impl

    #[derive(diesel::deserialize::FromSqlRow, diesel::expression::AsExpression)]
    #[diesel(sql_type = diesel::sql_types::#sql_type)]
    #orig_input
  };

  output.into()
}

fn process_text_enum(
  orig_input: TokenStream2,
  enum_name: &Ident,
  enum_name_str: String,
  variants_data: Vec<VariantData>,
  attributes: Attributes,
) -> TokenStream {
  let Attributes {
    table: table_name,
    column: column_name,
    conn: check,
    ..
  } = attributes;

  let conversion_to_string = traverse_enum(&variants_data, |variant| {
    let db_name = &variant.db_name;
    let variant_ident = &variant.ident;

    quote! {
      Self::#variant_ident => #db_name,
    }
  });

  let conversion_from_str = traverse_enum(&variants_data, |variant| {
    let db_name = &variant.db_name;
    let variant_ident = &variant.ident;

    quote! {
      #db_name => Ok(Self::#variant_ident),
    }
  });

  let test_impl = match check {
    Check::Conn(connection_func) => {
      let table_name = table_name.unwrap_or_else(|| enum_name_str.to_case(Case::Snake));
      let table_name_ident = format_ident!("{table_name}");

      let column_name = column_name.unwrap_or_else(|| "name".to_string());
      let column_name_ident = format_ident!("{column_name}");

      let test_mod_name =
        format_ident!("__diesel_enum_test_{}", enum_name_str.to_case(Case::Snake));

      let test_func_name = format_ident!("diesel_enum_test_{}", enum_name_str.to_case(Case::Snake));

      let variant_db_names = variants_data.iter().filter_map(|data| {
        if !data.skip_check {
          Some(&data.db_name)
        } else {
          None
        }
      });

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
          #conversion_from_str
          _ => Err(Box::from(format!("Unknown {}: {}", stringify!(#enum_name), value))),
        }
      }
    }

    impl diesel::serialize::ToSql<diesel::sql_types::Text, diesel::sqlite::Sqlite> for #enum_name {
      fn to_sql<'b>(&'b self, out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>) -> diesel::serialize::Result {
        let value = match self {
          #conversion_to_string
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
