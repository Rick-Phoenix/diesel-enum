#![allow(clippy::too_many_arguments)]

use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use syn::Ident;

use crate::{attributes::NameTypes, features::async_tests, TokenStream2, VariantData};

pub fn test_with_id(
  enum_name: &Ident,
  enum_name_str: &str,
  table_path: &TokenStream2,
  table_name: &str,
  column_name: &str,
  id_rust_type: &Ident,
  conn_callback: &TokenStream2,
  variants_data: &[VariantData],
  skip_test: bool,
) -> TokenStream2 {
  let column_name_ident = format_ident!("{column_name}");

  let test_mod_name = format_ident!("__diesel_enum_test_{}", enum_name_str.to_case(Case::Snake));

  let variants_map = {
    let mut collection_tokens = TokenStream2::new();

    let variants_map_ident = format_ident!("map");

    for variant in variants_data {
      let db_name = &variant.db_name;
      let variant_ident = &variant.ident;

      collection_tokens.extend(quote! {
        #variants_map_ident.insert(#db_name, #enum_name::#variant_ident.into());
      });
    }

    quote! {
      let mut #variants_map_ident: HashMap<&'static str, #id_rust_type> = HashMap::new();

      #collection_tokens

      #variants_map_ident
    }
  };

  let (async_fn, await_call) = if async_tests() {
    (Some(quote! { async }), Some(quote! { .await }))
  } else {
    (None, None)
  };

  let auto_test = if !skip_test {
    let test_label = if async_tests() {
      quote! { #[tokio::test] }
    } else {
      quote! { #[test] }
    };

    let test_func_name = format_ident!("diesel_enum_test_{}", enum_name_str.to_case(Case::Snake));

    Some(quote! {
      #test_label
      #async_fn fn #test_func_name() {
        #enum_name::check_consistency()#await_call.unwrap();
      }
    })
  } else {
    None
  };

  quote! {
    #[cfg(test)]
    mod #test_mod_name {
      use super::*;
      use diesel::prelude::*;
      use std::collections::HashMap;

      impl #enum_name {
        #[track_caller]
        pub #async_fn fn check_consistency() -> Result<(), diesel_enums::DbEnumError>
        {
          #conn_callback(|conn| {
            let enum_name = #enum_name_str;
            let table_name = #table_name;
            let column_name = #column_name;

            let mut rust_variants: HashMap<&'static str, #id_rust_type> = {
              #variants_map
            };

            let db_variants: Vec<(#id_rust_type, String)> = #table_path::table
              .select((#table_path::id, #table_path::#column_name_ident))
              .load(conn)
              .unwrap_or_else(|e| panic!("\n ❌ Failed to load the variants for the rust enum `{enum_name}` from the database column `{table_name}.{column_name}`: {e}"));

            let mut missing_variants: Vec<String> = Vec::new();

            let mut id_mismatches: Vec<(String, i64, i64)> = Vec::new();

            for (id, name) in db_variants {
              let variant_id = if let Some(variant) = rust_variants.remove(name.as_str()) {
                variant
              } else {
                missing_variants.push(name);
                continue;
              };

              if id != variant_id {
                id_mismatches.push((name, id as i64, variant_id as i64));
              }
            }

            if !missing_variants.is_empty() || !rust_variants.is_empty() || !id_mismatches.is_empty() {
              let mut error = diesel_enums::DbEnumError::new(enum_name.to_string(), diesel_enums::DbEnumSource::Column { table: table_name.to_string(), column: column_name.to_string() });

              if !id_mismatches.is_empty() {
                error.errors.push(diesel_enums::ErrorKind::IdMismatches(id_mismatches));
              }

              if !missing_variants.is_empty() {
                missing_variants.sort();

                error.errors.push(diesel_enums::ErrorKind::MissingFromRustEnum(missing_variants));
              }

              if !rust_variants.is_empty() {
                let mut excess_variants: Vec<String> = rust_variants.into_iter().map(|(name, _)| name.to_string()).collect();
                excess_variants.sort();

                error.errors.push(diesel_enums::ErrorKind::MissingFromDb(excess_variants));
              }

              Err(error)
            } else {
              Ok(())
            }
          })#await_call
        }
      }

      #auto_test
    }
  }
}

pub fn test_without_id(
  enum_name: &Ident,
  enum_name_str: &str,
  table_path: &TokenStream2,
  table_name: &str,
  column_name: &str,
  db_type: &NameTypes,
  conn_callback: &TokenStream2,
  variants_data: &[VariantData],
  skip_test: bool,
) -> TokenStream2 {
  let (names_query, source_type) = if let NameTypes::Custom { name: db_enum_name } = db_type {
    (
      quote! {
        #[derive(diesel::deserialize::QueryableByName)]
        struct DbEnum {
          #[diesel(sql_type = diesel::sql_types::Text)]
          pub variant: String
        }

        let result: Vec<DbEnum> = diesel::sql_query(concat!(r#"SELECT unnest(enum_range(NULL::"#, #db_enum_name, ")) AS variant"))
          .load(conn)
          .unwrap_or_else(|e| panic!("\n ❌ Failed to load the variants for the rust enum `{enum_name}` from the database enum `{}`: {e}", #db_enum_name));

        result.into_iter().map(|res| res.variant).collect()
      },
      quote! { diesel_enums::DbEnumSource::CustomEnum(#db_enum_name.to_string()) },
    )
  } else {
    let column_name_ident = format_ident!("{column_name}");

    (
      quote! {
        #table_path::table
          .select(#table_path::#column_name_ident)
          .load(conn)
          .unwrap_or_else(|e| panic!("\n ❌ Failed to load the variants for the rust enum `{enum_name}` from the database column `{}.{}`: {e}", #table_name, #column_name))
      },
      quote! { diesel_enums::DbEnumSource::Column { table: #table_name.to_string(), column: #column_name.to_string() } },
    )
  };

  let test_mod_name = format_ident!("__diesel_enum_test_{}", enum_name_str.to_case(Case::Snake));

  let variant_db_names = variants_data.iter().map(|data| &data.db_name);

  let (async_fn, await_call) = if async_tests() {
    (Some(quote! { async }), Some(quote! { .await }))
  } else {
    (None, None)
  };

  let auto_test = if !skip_test {
    let test_label = if async_tests() {
      quote! { #[tokio::test] }
    } else {
      quote! { #[test] }
    };

    let test_func_name = format_ident!("diesel_enum_test_{}", enum_name_str.to_case(Case::Snake));

    Some(quote! {
      #test_label
      #async_fn fn #test_func_name() {
        #enum_name::check_consistency()#await_call.unwrap();
      }
    })
  } else {
    None
  };

  quote! {
    #[cfg(test)]
    mod #test_mod_name {
      use super::*;
      use diesel::prelude::*;
      use std::collections::HashSet;

      impl #enum_name {
        #[track_caller]
        pub #async_fn fn check_consistency() -> Result<(), diesel_enums::DbEnumError>
        {
          #conn_callback(|conn| {
            let enum_name = #enum_name_str;

            let mut rust_variants = HashSet::from({
              [ #(#variant_db_names),* ]
            });

            let db_variants: Vec<String> = {
              #names_query
            };

            let mut missing_variants: Vec<String> = Vec::new();

            for variant in db_variants {
              let was_present = rust_variants.remove(variant.as_str());

              if !was_present {
                missing_variants.push(variant);
              }
            }

            if !missing_variants.is_empty() || !rust_variants.is_empty() {
              let mut error = diesel_enums::DbEnumError::new(enum_name.to_string(), #source_type);

              if !missing_variants.is_empty() {
                missing_variants.sort();

                error.errors.push(diesel_enums::ErrorKind::MissingFromRustEnum(missing_variants));
              }

              if !rust_variants.is_empty() {
                let mut excess_variants: Vec<String> = rust_variants.into_iter().map(|name| name.to_string()).collect();
                excess_variants.sort();

                error.errors.push(diesel_enums::ErrorKind::MissingFromDb(excess_variants));
              }

              Err(error)
            } else {
              Ok(())
            }
          })#await_call
        }
      }

      #auto_test
    }
  }
}

pub fn check_consistency_inter_call(enum_name: &Ident) -> TokenStream2 {
  let (async_fn, await_call) = if async_tests() {
    (Some(quote! { async }), Some(quote! { .await }))
  } else {
    (None, None)
  };

  let id_enum = format_ident!("{enum_name}Id");

  quote! {
    #[cfg(test)]
    impl #enum_name {
      #[track_caller]
      pub #async_fn fn check_consistency() -> Result<(), diesel_enums::DbEnumError> {
        #id_enum::check_consistency()#await_call
      }
    }
  }
}
