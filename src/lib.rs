#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

pub(crate) mod features;
#[macro_use]
pub(crate) mod macros;
pub(crate) mod attributes;
pub(crate) mod conversions;
pub(crate) mod process_variants;
pub(crate) mod test_generation;

use proc_macro::TokenStream;
pub(crate) use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Error, ItemEnum, Path};

use crate::{
  attributes::{Attributes, IdMapping, NameMapping},
  conversions::{
    enum_int_conversions, enum_to_enum_conversion, sql_int_conversions, sql_string_conversions,
  },
  process_variants::{process_variants, VariantData},
  test_generation::{test_with_id, test_without_id},
};

enum Check {
  Conn(Path),
  Skip,
}

fn traverse_enum<T>(variants: &[VariantData], action: T) -> TokenStream2
where
  T: Fn(&VariantData) -> TokenStream2,
{
  let mut tokens = TokenStream2::new();

  for variant in variants {
    let action_tokens = action(variant);
    tokens.extend(action_tokens);
  }

  tokens
}

#[proc_macro_attribute]
pub fn diesel_enum(attrs: TokenStream, input: TokenStream) -> TokenStream {
  let orig_input: TokenStream2 = input.clone().into();

  let attributes = parse_macro_input!(attrs as Attributes);

  let ast = parse_macro_input!(input as ItemEnum);

  let variants_data = match process_variants(&ast.variants, attributes.case) {
    Ok(data) => data,
    Err(e) => return e.to_compile_error().into(),
  };

  let enum_name = &ast.ident;
  let enum_name_str = enum_name.to_string();

  let mut enum_impls = TokenStream2::new();

  if let Check::Conn(connection_func) = attributes.conn {
    let test_impl = if let Some(IdMapping { rust_type, .. }) = &attributes.id_mapping {
      test_with_id(
        &enum_name_str,
        attributes.table,
        attributes.column,
        &rust_type,
        &connection_func,
        &variants_data,
      )
    } else if let Some(NameMapping { db_type, .. }) = &attributes.name_mapping {
      test_without_id(
        &enum_name_str,
        attributes.table,
        attributes.column,
        &db_type,
        &connection_func,
        &variants_data,
      )
    } else {
      return Error::new_spanned(
        orig_input,
        "At least one between `id_mapping` and `name_mapping` must be set",
      )
      .to_compile_error()
      .into();
    };

    enum_impls.extend(test_impl);
  }

  if let Some(IdMapping {
    type_path: sql_type_path,
    auto_increment,
    rust_type,
  }) = &attributes.id_mapping
  {
    let has_double_mapping = attributes.name_mapping.is_some();

    let target_enum_name = if has_double_mapping {
      format_ident!("{enum_name}Id")
    } else {
      enum_name.clone()
    };

    let int_to_from_sql = sql_int_conversions(
      &target_enum_name,
      &rust_type,
      &sql_type_path,
      &variants_data,
    );

    enum_impls.extend(int_to_from_sql);

    if *auto_increment {
      let int_conversion = enum_int_conversions(&target_enum_name, &rust_type, &variants_data);

      enum_impls.extend(int_conversion);
    }

    if !has_double_mapping {
      enum_impls.extend(quote! {
        #[derive(diesel::deserialize::FromSqlRow, diesel::expression::AsExpression)]
        #[diesel(sql_type = #sql_type_path)]
        #orig_input
      });
    } else {
      let enum_to_enum_conversion_tokens = enum_to_enum_conversion(&enum_name, &variants_data);

      let mut enum_copy = ast.clone();

      enum_copy.ident = target_enum_name;

      enum_copy
        .attrs
        .retain(|att| !att.path().is_ident("diesel_enum"));

      for variant in enum_copy.variants.iter_mut() {
        variant
          .attrs
          .retain(|att| !att.path().is_ident("diesel_enum"));
      }

      enum_impls.extend(quote! {
        #[derive(diesel::deserialize::FromSqlRow, diesel::expression::AsExpression)]
        #[diesel(sql_type = #sql_type_path)]
        #enum_copy

        #enum_to_enum_conversion_tokens
      });
    }
  }

  if let Some(NameMapping {
    path: sql_type_path,
    ..
  }) = attributes.name_mapping
  {
    let sql_string_conversions = sql_string_conversions(&enum_name, &sql_type_path, &variants_data);

    enum_impls.extend(sql_string_conversions);

    enum_impls.extend(quote! {
      #[derive(diesel::deserialize::FromSqlRow, diesel::expression::AsExpression)]
      #[diesel(sql_type = #sql_type_path)]
      #orig_input
    });
  }

  enum_impls.into()
}
