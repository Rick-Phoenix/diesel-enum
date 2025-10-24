use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{Ident, LitByteStr, LitInt};

use crate::{traverse_enum, TokenStream2, VariantData};

pub fn enum_int_conversions(
  enum_name: &Ident,
  rust_type: &Ident,
  variants_data: &[VariantData],
) -> TokenStream2 {
  let mut into_int = TokenStream2::new();

  let mut from_int = TokenStream2::new();

  for variant in variants_data {
    let id = LitInt::new(&format!("{}{}", variant.id, rust_type), Span::call_site());
    let variant_ident = &variant.ident;

    into_int.extend(quote! {
      Self::#variant_ident => #id,
    });

    from_int.extend(quote! {
      #id => Ok(Self::#variant_ident),
    });
  }

  quote! {
    impl TryFrom<#rust_type> for #enum_name {
      type Error = String;

      fn try_from(value: #rust_type) -> Result<Self, Self::Error> {
        match value {
          #from_int
          x => Err(format!("Unknown `{}` variant: {x}", stringify!(#enum_name))),
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
  }
}

pub fn sql_int_conversions(
  enum_name: &Ident,
  rust_type: &Ident,
  sql_type_path: &TokenStream2,
  variants_data: &[VariantData],
) -> TokenStream2 {
  let to_sql_conversion = traverse_enum(variants_data, |data| {
    let variant = &data.ident;
    let id = LitInt::new(&format!("{}{}", data.id, rust_type), Span::call_site());

    quote! {
      Self::#variant => #id.to_sql(out),
    }
  });

  quote! {
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
  }
}

pub fn postgres_enum_conversions(
  enum_name: &Ident,
  sql_type_path: &TokenStream2,
  variants_data: &[VariantData],
) -> TokenStream2 {
  let mut conversion_to_bytes = TokenStream2::new();
  let mut conversion_from_bytes = TokenStream2::new();

  for data in variants_data {
    let db_name_bytes = LitByteStr::new(data.db_name.as_bytes(), Span::call_site());
    let variant_ident = &data.ident;

    conversion_to_bytes.extend(quote! {
      Self::#variant_ident => out.write_all(#db_name_bytes)?,
    });

    conversion_from_bytes.extend(quote! {
      #db_name_bytes => Ok(Self::#variant_ident),
    });
  }

  quote! {
    impl diesel::deserialize::FromSql<#sql_type_path, diesel::pg::Pg> for #enum_name
    {
      fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        match bytes.as_bytes() {
          #conversion_from_bytes
          unknown => Err(Box::from(format!("Unknown `{}` variant: {}", stringify!(#enum_name), String::from_utf8_lossy(unknown)))),
        }
      }
    }

    impl diesel::serialize::ToSql<#sql_type_path, diesel::pg::Pg> for #enum_name
    {
      fn to_sql<'b>(&'b self, out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        use std::io::Write;
        match *self {
          #conversion_to_bytes
        };
        Ok(diesel::serialize::IsNull::No)
      }
    }
  }
}

pub fn to_from_str_conversions(enum_name: &Ident, variants_data: &[VariantData]) -> TokenStream2 {
  let mut conversion_to_str = TokenStream2::new();
  let mut conversion_from_str = TokenStream2::new();

  for data in variants_data {
    let db_name = &data.db_name;
    let variant_ident = &data.ident;

    conversion_to_str.extend(quote! {
      Self::#variant_ident => #db_name,
    });

    conversion_from_str.extend(quote! {
      #db_name => Ok(Self::#variant_ident),
    });
  }

  quote! {
    impl #enum_name {
      /// Returns the variant's corresponding name in the database source.
      pub fn db_name(&self) -> &'static str {
        match self {
          #conversion_to_str
        }
      }

      /// Returns the enum variant corresponding to a given name, if there is one.
      pub fn from_db_name(name: &str) -> Result<Self, String> {
        match name {
          #conversion_from_str
          _ => Err(format!("No matching {} variant found for `{name}`", stringify!(#enum_name)))
        }
      }
    }
  }
}

pub fn sql_string_conversions(enum_name: &Ident, sql_type_path: &TokenStream2) -> TokenStream2 {
  quote! {
    impl<DB> diesel::deserialize::FromSql<#sql_type_path, DB> for #enum_name
    where
      DB: diesel::backend::Backend,
      String: diesel::deserialize::FromSql<#sql_type_path, DB>,
    {
      fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
       let value = <String as diesel::deserialize::FromSql<#sql_type_path, DB>>::from_sql(bytes)?;

        Self::from_db_name(&value).map_err(Box::from)
      }
    }

    impl<DB> diesel::serialize::ToSql<#sql_type_path, DB> for #enum_name
    where
      DB: diesel::backend::Backend,
      str: diesel::serialize::ToSql<#sql_type_path, DB>,
    {
      fn to_sql<'b>(&'b self, out: &mut diesel::serialize::Output<'b, '_, DB>) -> diesel::serialize::Result {
        self.db_name().to_sql(out)
      }
    }
  }
}

pub fn enum_to_enum_conversion(enum_name: &Ident, variants_data: &[VariantData]) -> TokenStream2 {
  let id_enum = format_ident!("{enum_name}Id");

  let from_text_enum = traverse_enum(variants_data, |variant| {
    let variant_ident = &variant.ident;

    quote! {
      #enum_name::#variant_ident => #id_enum::#variant_ident,
    }
  });

  let from_id_enum = traverse_enum(variants_data, |variant| {
    let variant_ident = &variant.ident;

    quote! {
      #id_enum::#variant_ident => #enum_name::#variant_ident,
    }
  });

  quote! {
    impl From<#enum_name> for #id_enum {
      fn from(value: #enum_name) -> Self {
        match value {
          #from_text_enum
        }
      }
    }

    impl From<#id_enum> for #enum_name {
      fn from(value: #id_enum) -> Self {
        match value {
          #from_id_enum
        }
      }
    }
  }
}
