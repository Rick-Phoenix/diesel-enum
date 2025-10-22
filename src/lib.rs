#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

pub(crate) mod features;
#[macro_use]
pub(crate) mod macros;
pub(crate) mod attributes;
pub(crate) mod conversions;
pub(crate) mod process_variants;
pub(crate) mod test_generation;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
pub(crate) use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Error, ItemEnum};

use crate::{
  attributes::{Attributes, IdMapping, NameMapping},
  conversions::{
    enum_int_conversions, enum_to_enum_conversion, sql_int_conversions, sql_string_conversions,
  },
  process_variants::{process_variants, VariantData},
  test_generation::{check_consistency_inter_call, test_with_id, test_without_id},
};

enum Check {
  Conn(TokenStream2),
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

  let Attributes {
    table_path,
    skip_test,
    table_name,
    column,
    conn,
    case,
    name_mapping,
    id_mapping,
    skip_ranges,
  } = parse_macro_input!(attrs as Attributes);

  let ast = parse_macro_input!(input as ItemEnum);

  let variants_data = match process_variants(&ast.variants, case, &skip_ranges) {
    Ok(data) => data,
    Err(e) => return e.to_compile_error().into(),
  };

  let enum_name = &ast.ident;
  let enum_name_str = enum_name.to_string();

  let table_name = table_name.unwrap_or_else(|| enum_name_str.to_case(Case::Snake));
  let table_path = table_path.unwrap_or_else(|| {
    let table_name_ident = format_ident!("{table_name}");
    quote! { crate::schema::#table_name_ident }
  });
  let column_name = column.as_deref().unwrap_or_else(|| "name");

  let mut enum_impls = TokenStream2::new();

  if id_mapping.is_none() && name_mapping.is_none() {
    return Error::new_spanned(
      orig_input,
      "At least one between `id_mapping` and `name_mapping` must be set",
    )
    .to_compile_error()
    .into();
  }

  if let Some(NameMapping {
    path: sql_type_path,
    db_type,
  }) = &name_mapping
  {
    enum_impls.extend(quote! {
      #[derive(PartialEq, Eq, Clone, Copy, Hash, diesel_enum_checked::MappedEnum, Debug, diesel::deserialize::FromSqlRow, diesel::expression::AsExpression)]
      #[diesel(sql_type = #sql_type_path)]
      #orig_input
    });

    let sql_string_conversions = sql_string_conversions(&enum_name, &sql_type_path, &variants_data);

    enum_impls.extend(sql_string_conversions);

    if let Check::Conn(connection_func) = &conn {
      let test_impl = if id_mapping.is_none() {
        test_without_id(
          &enum_name,
          &enum_name_str,
          &table_path,
          &table_name,
          &column_name,
          &db_type,
          &connection_func,
          &variants_data,
          skip_test,
        )
      } else {
        check_consistency_inter_call(&enum_name)
      };

      enum_impls.extend(test_impl);
    }
  }

  if let Some(IdMapping {
    type_path: sql_type_path,
    rust_type,
  }) = id_mapping
  {
    let has_double_mapping = name_mapping.is_some();

    let target_enum_name = if has_double_mapping {
      format_ident!("{enum_name}Id")
    } else {
      enum_name.clone()
    };

    let target_enum_str = target_enum_name.to_string();

    let int_to_from_sql = sql_int_conversions(
      &target_enum_name,
      &rust_type,
      &sql_type_path,
      &variants_data,
    );

    enum_impls.extend(int_to_from_sql);

    let int_conversion = enum_int_conversions(&target_enum_name, &rust_type, &variants_data);

    enum_impls.extend(int_conversion);

    if let Check::Conn(connection_func) = &conn {
      let test_impl = test_with_id(
        &target_enum_name,
        &target_enum_str,
        &table_path,
        &table_name,
        &column_name,
        &rust_type,
        &connection_func,
        &variants_data,
        skip_test,
      );

      enum_impls.extend(test_impl);
    }

    if !has_double_mapping {
      enum_impls.extend(quote! {
        #[derive(PartialEq, Eq, Clone, Copy, Hash, Debug, diesel_enum_checked::MappedEnum, diesel::deserialize::FromSqlRow, diesel::expression::AsExpression)]
        #[diesel(sql_type = #sql_type_path)]
        #orig_input
      });
    } else {
      let enum_to_enum_conversion_tokens = enum_to_enum_conversion(&enum_name, &variants_data);

      let mut enum_copy = ast.clone();

      enum_copy.ident = target_enum_name;

      enum_impls.extend(quote! {
        #[derive(PartialEq, Eq, Clone, Copy, Hash, Debug, diesel_enum_checked::MappedEnum, diesel::deserialize::FromSqlRow, diesel::expression::AsExpression)]
        #[diesel(sql_type = #sql_type_path)]
        #enum_copy

        #enum_to_enum_conversion_tokens
      });
    }
  }

  enum_impls.into()
}

#[proc_macro_derive(MappedEnum, attributes(db_mapping))]
pub fn derive_macro(_input: TokenStream) -> TokenStream {
  TokenStream::new()
}
