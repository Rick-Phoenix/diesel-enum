use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use syn::{Ident, Path};

use crate::{
  attributes::NameTypes,
  features::{async_tests, pretty_test_errors},
  TokenStream2, VariantData,
};

pub fn test_with_id(
  enum_name: &Ident,
  enum_name_str: &str,
  table_name: &str,
  column_name: &str,
  id_rust_type: &Ident,
  conn_callback: &Path,
  variants_data: &[VariantData],
) -> TokenStream2 {
  let table_name_ident = format_ident!("{table_name}");
  let column_name_ident = format_ident!("{column_name}");

  let test_mod_name = format_ident!("__diesel_enum_test_{}", enum_name_str.to_case(Case::Snake));
  let test_func_name = format_ident!("diesel_enum_test_{}", enum_name_str.to_case(Case::Snake));

  let variants_map = {
    let mut collection_tokens = TokenStream2::new();

    let variants_map_ident = format_ident!("map");

    for variant in variants_data {
      if !variant.skip_check {
        let db_name = &variant.db_name;
        let variant_ident = &variant.ident;

        collection_tokens.extend(quote! {
          #variants_map_ident.insert(#db_name, #enum_name::#variant_ident.into());
        });
      }
    }

    quote! {
      let mut #variants_map_ident: HashMap<&'static str, #id_rust_type> = HashMap::new();

      #collection_tokens

      #variants_map_ident
    }
  };

  let (test_label, async_fn, await_call) = if async_tests() {
    (
      quote! { #[tokio::test] },
      Some(quote! { async }),
      Some(quote! { .await }),
    )
  } else {
    (quote! { #[test] }, None, None)
  };

  let desync_error_message = if pretty_test_errors() {
    quote! {
      writeln!(error_message, "\n ❌ The rust enum `{}` and the database column `{}.{}` are out of sync: ", enum_name.bright_yellow(), table_name.bright_cyan(), column_name.bright_cyan()).unwrap();
    }
  } else {
    quote! {
      writeln!(error_message, "\n ❌ The rust enum `{enum_name}` and the database column `{table_name}.{column_name}` are out of sync: ").unwrap();
    }
  };

  let missing_rust_variants_error = if pretty_test_errors() {
    quote! {
      writeln!(error_message, "\n  - Variants missing from the {}:", "rust enum".bright_yellow()).unwrap();
      for variant in &missing_variants {
        writeln!(error_message, "    • {variant}").unwrap();
      }
    }
  } else {
    quote! {
      writeln!(error_message, "\n  - Variants missing from the rust enum: [ {} ]", missing_variants.join(", ")).unwrap();
    }
  };

  let missing_db_variants_error = if pretty_test_errors() {
    quote! {
      writeln!(error_message, "\n  - Variants missing from the {}:", "database".bright_cyan()).unwrap();
      for variant in &excess_variants {
        writeln!(error_message, "    • {variant}").unwrap();
      }
    }
  } else {
    quote! {
      writeln!(error_message, "\n  - Variants missing from the database: [ {} ]", excess_variants.join(", ")).unwrap();
    }
  };

  let id_mismatch_error = if pretty_test_errors() {
    quote! {
      writeln!(error_message, "\n  - Wrong id mapping for `{}`", name.bright_yellow()).unwrap();
      writeln!(error_message, "    Expected: {}", expected.bright_green()).unwrap();
      writeln!(error_message, "    Found: {}", found.bright_red()).unwrap();
    }
  } else {
    quote! {
      writeln!(error_message, "\n  - Wrong id mapping for `{name}`").unwrap();
      writeln!(error_message, "    Expected: {expected}").unwrap();
      writeln!(error_message, "    Found: {found}").unwrap();
    }
  };

  let owo_import = if pretty_test_errors() {
    Some(quote! { use owo_colors::OwoColorize; })
  } else {
    None
  };

  quote! {
    #[cfg(test)]
    mod #test_mod_name {
      use super::*;
      use diesel::prelude::*;
      use std::collections::HashMap;
      use crate::schema::#table_name_ident;
      use std::fmt::Write;
      #owo_import

      #test_label
      #async_fn fn #test_func_name() {
        #conn_callback(|conn| {
          let enum_name = #enum_name_str;
          let table_name = #table_name;
          let column_name = #column_name;

          let mut rust_variants: HashMap<&'static str, #id_rust_type> = {
            #variants_map
          };

          let db_variants: Vec<(#id_rust_type, String)> = #table_name_ident::table
            .select((#table_name_ident::id, #table_name_ident::#column_name_ident))
            .load(conn)
            .unwrap_or_else(|e| panic!("\n ❌ Failed to load the variants for the rust enum `{enum_name}` from the database column `{table_name}.{column_name}`: {e}"));

          let mut missing_variants: Vec<String> = Vec::new();

          let mut id_mismatches: Vec<(String, #id_rust_type, #id_rust_type)> = Vec::new();

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

            #desync_error_message

            for ((name, expected, found)) in id_mismatches {
              #id_mismatch_error
            }

            if !missing_variants.is_empty() {
              missing_variants.sort();

              #missing_rust_variants_error
            }

            if !rust_variants.is_empty() {
              let mut excess_variants: Vec<&str> = rust_variants.into_iter().map(|(name, _)| name).collect();
              excess_variants.sort();

              #missing_db_variants_error
            }

            panic!("{error_message}");
          }
        })#await_call;
      }
    }
  }
}

pub fn test_without_id(
  enum_name: &str,
  table_name: &str,
  column_name: &str,
  db_type: &NameTypes,
  conn_callback: &Path,
  variants_data: &[VariantData],
) -> TokenStream2 {
  let is_custom = db_type.is_custom();

  let (names_query, target_name) = if let NameTypes::Custom { name: db_enum_name } = db_type {
    (
      quote! {
        #[derive(diesel::deserialize::QueryableByName)]
        struct DbEnum {
          #[diesel(sql_type = diesel::sql_types::Text)]
          pub variant: String
        }

        diesel::sql_query(concat!(r#"SELECT unnest(enum_range(NULL::"#, #db_enum_name, ")) AS variant"))
      },
      db_enum_name.clone(),
    )
  } else {
    let table_name_ident = format_ident!("{table_name}");
    let column_name_ident = format_ident!("{column_name}");

    (
      quote! {
        crate::schema::#table_name_ident::table
          .select(crate::schema::#table_name_ident::#column_name_ident)
      },
      format!("{table_name}.{column_name}"),
    )
  };

  let test_mod_name = format_ident!("__diesel_enum_test_{}", enum_name.to_case(Case::Snake));
  let test_func_name = format_ident!("diesel_enum_test_{}", enum_name.to_case(Case::Snake));

  let variant_db_names = variants_data.iter().filter_map(|data| {
    if !data.skip_check {
      Some(&data.db_name)
    } else {
      None
    }
  });

  let source_type = if is_custom { "enum" } else { "column" };

  let (test_label, async_fn, await_call) = if async_tests() {
    (
      quote! { #[tokio::test] },
      Some(quote! { async }),
      Some(quote! { .await }),
    )
  } else {
    (quote! { #[test] }, None, None)
  };

  let desync_error_message = if pretty_test_errors() {
    quote! {
      writeln!(error_message, "\n ❌ The rust enum `{}` and the database {source_type} `{}` are out of sync: ", enum_name.bright_yellow(), target_name.bright_cyan()).unwrap();
    }
  } else {
    quote! {
      writeln!(error_message, "\n ❌ The rust enum `{enum_name}` and the database {source_type} `{target_name}` are out of sync: ").unwrap();
    }
  };

  let missing_rust_variants_error = if pretty_test_errors() {
    quote! {
      writeln!(error_message, "\n  - Variants missing from the {}:", "rust enum".bright_yellow()).unwrap();
      for variant in &missing_variants {
        writeln!(error_message, "    • {variant}").unwrap();
      }
    }
  } else {
    quote! {
      writeln!(error_message, "\n  - Variants missing from the rust enum: [ {} ]", missing_variants.join(", ")).unwrap();
    }
  };

  let missing_db_variants_error = if pretty_test_errors() {
    quote! {
      writeln!(error_message, "\n  - Variants missing from the {}:", "database".bright_cyan()).unwrap();
      for variant in &excess_variants {
        writeln!(error_message, "    • {variant}").unwrap();
      }
    }
  } else {
    quote! {
      writeln!(error_message, "\n  - Variants missing from the database: [ {} ]", excess_variants.join(", ")).unwrap();
    }
  };

  let owo_import = if pretty_test_errors() {
    Some(quote! { use owo_colors::OwoColorize; })
  } else {
    None
  };

  quote! {
    #[cfg(test)]
    mod #test_mod_name {
      use super::*;
      use diesel::prelude::*;
      use std::collections::HashSet;
      use std::fmt::Write;
      #owo_import

      #test_label
      #async_fn fn #test_func_name() {
        #conn_callback(|conn| {
          let source_type = #source_type;
          let enum_name = #enum_name;
          let target_name = #target_name;

          let mut rust_variants = HashSet::from({
            [ #(#variant_db_names),* ]
          });

          let query = {
            #names_query
          };

          let db_variants: Vec<String> = query
          .load(conn)
          .unwrap_or_else(|e| panic!("\n ❌ Failed to load the variants for the rust enum `{enum_name}` from the database {source_type} `{target_name}`: {e}"));

          let mut missing_variants: Vec<String> = Vec::new();

          for variant in db_variants {
            let was_present = rust_variants.remove(variant.as_str());

            if !was_present {
              missing_variants.push(variant);
            }
          }

          if !missing_variants.is_empty() || !rust_variants.is_empty() {
            let mut error_message = String::new();

            #desync_error_message

            if !missing_variants.is_empty() {
              missing_variants.sort();

              #missing_rust_variants_error
            }

            if !rust_variants.is_empty() {
              let mut excess_variants: Vec<&str> = rust_variants.into_iter().collect();
              excess_variants.sort();

              #missing_db_variants_error
            }

            panic!("{error_message}");
          }
        })#await_call;
      }
    }
  }
}
