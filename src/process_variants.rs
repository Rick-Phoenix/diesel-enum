use std::{cmp::Ordering, ops::Range};

use convert_case::{Case, Casing};
use syn::{punctuated::Punctuated, Error, Ident, LitInt, LitStr, Token, Variant};

pub struct VariantData {
  pub ident: Ident,
  pub db_name: String,
  pub id: i32,
}

fn is_skipped(id: i32, sorted_ranges: &[Range<i32>]) -> bool {
  let result = sorted_ranges.binary_search_by(|range| {
    if range.contains(&id) {
      Ordering::Equal
    } else if id < range.start {
      Ordering::Greater
    } else {
      Ordering::Less
    }
  });

  result.is_ok()
}

pub fn process_variants(
  variants: &Punctuated<Variant, Token![,]>,
  case: Case,
  skip_ranges: &[Range<i32>],
) -> Result<Vec<VariantData>, Error> {
  let mut variants_data: Vec<VariantData> = Vec::new();

  let mut current_id = 1;

  for variant in variants {
    let ident = variant.ident.clone();
    let mut db_name: Option<String> = None;
    let mut id: Option<i32> = None;

    for attr in &variant.attrs {
      if attr.meta.path().is_ident("db_mapping") {
        attr.parse_nested_meta(|meta| {
          if meta.path.is_ident("id") {
            let val = meta.value()?;
            id = Some(val.parse::<LitInt>()?.base10_parse::<i32>()?);
          } else if meta.path.is_ident("name") {
            let val = meta.value()?;

            db_name = Some(val.parse::<LitStr>()?.value());
          } else {
            return Err(meta.error("Unknown attribute. Allowed attributes are: [ id, name ]"));
          }

          Ok(())
        })?
      }
    }

    let id = id.unwrap_or_else(|| {
      while is_skipped(current_id, skip_ranges) {
        current_id += 1;
      }

      let found_id = current_id;

      current_id += 1;

      found_id
    });

    variants_data.push(VariantData {
      ident,
      db_name: db_name.unwrap_or_else(|| variant.ident.to_string().to_case(case)),
      id,
    });
  }

  Ok(variants_data)
}
