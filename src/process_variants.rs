use convert_case::{Case, Casing};
use syn::{punctuated::Punctuated, Error, Ident, LitInt, LitStr, Token, Variant};

pub struct VariantData {
  pub ident: Ident,
  pub db_name: String,
  pub id: i64,
}

pub fn process_variants(
  variants: &Punctuated<Variant, Token![,]>,
  case: Case,
) -> Result<Vec<VariantData>, Error> {
  let mut variants_data: Vec<VariantData> = Vec::new();

  for (i, variant) in variants.iter().enumerate() {
    let ident = variant.ident.clone();
    let mut db_name: Option<String> = None;
    let mut id: Option<i64> = None;

    for attr in &variant.attrs {
      if attr.meta.path().is_ident("db_mapping") {
        attr.parse_nested_meta(|meta| {
          if meta.path.is_ident("id") {
            let val = meta.value()?;
            id = Some(val.parse::<LitInt>()?.base10_parse::<i64>()?);
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

    variants_data.push(VariantData {
      ident,
      db_name: db_name.unwrap_or_else(|| variant.ident.to_string().to_case(case)),
      id: id.unwrap_or((i + 1) as i64),
    });
  }

  Ok(variants_data)
}
