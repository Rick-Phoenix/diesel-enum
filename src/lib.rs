#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

pub(crate) mod features;
#[macro_use]
pub(crate) mod macros;
pub(crate) mod attributes;
pub(crate) mod process_variants;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Span;
pub(crate) use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Ident, ItemEnum, LitInt, Path};

use crate::{
  attributes::Attributes,
  process_variants::{process_variants, VariantData},
};

enum Check {
  Conn(Path),
  Skip,
}

struct SqlType {
  pub db_type: MappedType,
  pub path: TokenStream2,
}

#[derive(Clone, Copy)]
enum MappedType {
  Text,
  I32,
  I64,
  I16,
  I8,
  Custom,
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

  let ast = parse_macro_input!(input as ItemEnum);

  let variants_data = match process_variants(ast.variants, attributes.case) {
    Ok(data) => data,
    Err(e) => return e.to_compile_error().into(),
  };

  let enum_name = &ast.ident;
  let enum_name_str = enum_name.to_string();

  match &attributes.sql_type.db_type {
    MappedType::Text | MappedType::Custom => process_text_enum(
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
    sql_type,
    conn: check,
    auto_increment,
    ..
  } = attributes;

  let rust_type = match &sql_type.db_type {
    MappedType::I16 => format_ident!("i16"),
    MappedType::I32 => format_ident!("i32"),
    MappedType::I64 => format_ident!("i64"),
    MappedType::I8 => format_ident!("i8"),
    _ => unreachable!(),
  };

  let sql_type_path = sql_type.path;

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
            x => Err(Box::from(format!("Unknown `{}` variant: {x}", stringify!(#enum_name)))),
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
            let enum_name = #enum_name_str;
            let table_name = #table_name;
            let column_name = #column_name;

            let mut rust_variants: HashMap<&'static str, #rust_type> = {
              #variants_map
            };

            let conn = &mut #connection_func();

            let db_variants: Vec<(#rust_type, String)> = #table_name_ident::table
              .select((#table_name_ident::id, #table_name_ident::#column_name_ident))
              .load(conn)
              .unwrap_or_else(|e| panic!("Failed to load the variants for the rust enum `{enum_name}` from the database column `{table_name}.{column_name}`: {e}"));

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

              write!(error_message, "The rust enum `{enum_name}` and the database column `{table_name}.{column_name}` are out of sync: ").unwrap();

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

  let to_sql_conversion = traverse_enum(&variants_data, |data| {
    let variant = &data.ident;
    let id = LitInt::new(&format!("{}{}", data.id, rust_type), Span::call_site());

    quote! {
      Self::#variant => #id.to_sql(out),
    }
  });

  let output = quote! {
    #int_conversion

    impl<DB> diesel::deserialize::FromSql<#sql_type_path, DB> for #enum_name
    where
      DB: diesel::backend::Backend,
      #rust_type: diesel::deserialize::FromSql<#sql_type_path, DB>,
    {
      fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let value = #rust_type::from_sql(bytes)?;

        Ok(value.try_into()?)
      }
    }

    impl<DB> diesel::serialize::ToSql<#sql_type_path, DB> for #enum_name
    where
      DB: diesel::backend::Backend,
      #rust_type: diesel::serialize::ToSql<#sql_type_path, DB>,
    {
      fn to_sql<'b>(&'b self, out: &mut diesel::serialize::Output<'b, '_, DB>) -> diesel::serialize::Result {
        match self {
          #to_sql_conversion
        }
      }
    }

    #test_impl

    #[derive(diesel::deserialize::FromSqlRow, diesel::expression::AsExpression)]
    #[diesel(sql_type = #sql_type_path)]
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
    name: db_enum_name,
    sql_type,
    ..
  } = attributes;

  let db_type = sql_type.db_type;

  let is_custom = matches!(db_type, MappedType::Custom);

  let sql_type_path = sql_type.path;

  let mut conversion_to_str = TokenStream2::new();
  let mut conversion_from_str = TokenStream2::new();

  for data in &variants_data {
    let db_name = &data.db_name;
    let variant_ident = &data.ident;

    conversion_to_str.extend(quote! {
      Self::#variant_ident => #db_name.to_sql(out),
    });

    conversion_from_str.extend(quote! {
      #db_name=> Ok(Self::#variant_ident),
    });
  }

  let test_impl = match check {
    Check::Conn(connection_func) => {
      let (names_query, target_name) = if !is_custom {
        let table_name = table_name.unwrap_or_else(|| enum_name_str.to_case(Case::Snake));
        let table_name_ident = format_ident!("{table_name}");

        let column_name = column_name.unwrap_or_else(|| "name".to_string());
        let column_name_ident = format_ident!("{column_name}");

        (
          quote! {
            crate::schema::#table_name_ident::table
              .select(crate::schema::#table_name_ident::#column_name_ident)
          },
          format!("{table_name}.{column_name}"),
        )
      } else {
        let db_enum_name = db_enum_name.unwrap_or_else(|| enum_name_str.to_case(Case::Snake));

        (
          quote! {
            #[derive(diesel::deserialize::QueryableByName)]
            struct DbEnum {
              #[diesel(sql_type = diesel::sql_types::Text)]
              pub variant: String
            }

            diesel::sql_query(concat!(r#"SELECT unnest(enum_range(NULL::"#, #db_enum_name, ")) AS variant"))
          },
          db_enum_name,
        )
      };

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

      let source_type = if is_custom { "enum" } else { "column" };

      Some(quote! {
        #[cfg(test)]
        mod #test_mod_name {
          use super::*;
          use diesel::prelude::*;
          use std::collections::HashSet;
          use std::fmt::Write;

          #[test]
          fn #test_func_name() {
            let source_type = #source_type;
            let enum_name = #enum_name_str;
            let target_name = #target_name;

            let mut rust_variants = HashSet::from({
              [ #(#variant_db_names),* ]
            });

            let conn = &mut #connection_func();

            let query = {
              #names_query
            };

            let db_variants: Vec<String> = query
              .load(conn)
              .unwrap_or_else(|e| panic!("Failed to load the variants for the rust enum `{enum_name}` from the database {source_type} `{target_name}`: {e}"));

            let mut missing_variants: Vec<String> = Vec::new();

            for variant in db_variants {
              let was_present = rust_variants.remove(variant.as_str());

              if !was_present {
                missing_variants.push(variant);
              }
            }

            if !missing_variants.is_empty() || !rust_variants.is_empty() {
              let mut error_message = String::new();

              write!(error_message, "The rust enum `{enum_name}` and the database {source_type} `{target_name}` are out of sync: ").unwrap();

              if !missing_variants.is_empty() {
                missing_variants.sort();

                write!(error_message, "\n  - Variants missing from the rust enum: [ {} ]", missing_variants.join(", ")).unwrap();
              }

              if !rust_variants.is_empty() {
                let mut excess_variants: Vec<&str> = rust_variants.into_iter().collect();
                excess_variants.sort();

                write!(error_message, "\n  - Variants missing from the database: [ {} ]",  excess_variants.join(", ")).unwrap();
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
    impl<DB> diesel::deserialize::FromSql<#sql_type_path, DB> for #enum_name
    where
      DB: diesel::backend::Backend,
      String: diesel::deserialize::FromSql<#sql_type_path, DB>,
    {
      fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
       let value = <String as diesel::deserialize::FromSql<#sql_type_path, DB>>::from_sql(bytes)?;

        match value.as_str() {
          #conversion_from_str
          x => Err(Box::from(format!("Unknown `{}` variant: {x}", stringify!(#enum_name)))),
        }
      }
    }

    impl<DB> diesel::serialize::ToSql<#sql_type_path, DB> for #enum_name
    where
      DB: diesel::backend::Backend,
      str: diesel::serialize::ToSql<#sql_type_path, DB>,
    {
      fn to_sql<'b>(&'b self, out: &mut diesel::serialize::Output<'b, '_, DB>) -> diesel::serialize::Result {
        match self {
          #conversion_to_str
        }
      }
    }

    #test_impl

    #[derive(diesel::deserialize::FromSqlRow, diesel::expression::AsExpression)]
    #[diesel(sql_type = #sql_type_path)]
    #orig_input
  };

  output.into()
}
