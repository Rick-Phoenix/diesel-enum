use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ErrorKind {
  MissingFromDb(Vec<String>),
  MissingFromRustEnum(Vec<String>),
  IdMismatches(Vec<(String, i64, i64)>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub struct DbEnumError {
  pub rust_enum: String,
  pub db_source: DbEnumSource,
  pub errors: Vec<ErrorKind>,
}

impl DbEnumError {
  pub fn new(rust_enum: String, db_source: DbEnumSource) -> Self {
    Self {
      rust_enum,
      db_source,
      errors: Vec::new(),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DbEnumSource {
  CustomEnum(String),
  Column { table: String, column: String },
}

impl DbEnumSource {
  pub fn name(&self) -> String {
    match self {
      Self::CustomEnum(name) => name.clone(),
      Self::Column { table, column } => format!("{table}.{column}"),
    }
  }

  pub fn db_type(&self) -> &str {
    match self {
      Self::CustomEnum(_) => "enum",
      Self::Column { .. } => "column",
    }
  }
}

#[cfg(feature = "pretty-test-errors")]
mod pretty_errors {
  use std::fmt::Display;

  use owo_colors::OwoColorize;

  use crate::{DbEnumError, ErrorKind};

  impl Display for DbEnumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      writeln!(
        f,
        "\n ❌ The rust enum `{}` and the database {} `{}` are out of sync: ",
        self.rust_enum.bright_yellow(),
        self.db_source.db_type(),
        self.db_source.name().bright_cyan()
      )
      .unwrap();

      for error in &self.errors {
        writeln!(f, "{}", error).unwrap();
      }

      Ok(())
    }
  }

  impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        ErrorKind::MissingFromDb(items) => {
          writeln!(
            f,
            "\n  - Variants missing from the {}:",
            "database".bright_cyan()
          )
          .unwrap();
          for variant in items {
            writeln!(f, "    • {variant}").unwrap();
          }
          Ok(())
        }
        ErrorKind::MissingFromRustEnum(items) => {
          writeln!(
            f,
            "\n  - Variants missing from the {}:",
            "rust enum".bright_yellow()
          )
          .unwrap();
          for variant in items {
            writeln!(f, "    • {variant}").unwrap();
          }
          Ok(())
        }
        ErrorKind::IdMismatches(items) => {
          for (name, expected, found) in items {
            writeln!(f, "\n  - Wrong id mapping for `{}`", name.bright_yellow()).unwrap();
            writeln!(f, "    Expected: {}", expected.bright_green()).unwrap();
            writeln!(f, "    Found: {}", found.bright_red()).unwrap();
          }
          Ok(())
        }
      }
    }
  }
}

#[cfg(not(feature = "pretty-test-errors"))]
mod standard_errors {
  use std::fmt::Display;

  use crate::{DbEnumError, ErrorKind};

  impl Display for DbEnumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      writeln!(
        f,
        "\n ❌ The rust enum `{}` and the database {} `{}` are out of sync: ",
        self.rust_enum,
        self.db_source.db_type(),
        self.db_source.name()
      )
      .unwrap();

      for error in &self.errors {
        writeln!(f, "{}", error).unwrap();
      }

      Ok(())
    }
  }

  impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        ErrorKind::MissingFromDb(items) => {
          writeln!(
            f,
            "\n  - Variants missing from the database: [ {} ]",
            items.join(", ")
          )
        }
        ErrorKind::MissingFromRustEnum(items) => {
          writeln!(
            f,
            "\n  - Variants missing from the rust enum: [ {} ]",
            items.join(", ")
          )
        }
        ErrorKind::IdMismatches(items) => {
          for (name, expected, found) in items {
            writeln!(
              f,
              "\n  - Wrong id mapping for `{name}`. Expected: {expected}, found: {found}"
            )
            .unwrap();
          }
          Ok(())
        }
      }
    }
  }
}
