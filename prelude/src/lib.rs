pub use diesel_enum_checked::*;

#[cfg(feature = "test-utils")]
mod test_runners;

#[cfg(feature = "test-utils")]
pub use test_runners::*;
//
use thiserror::Error;

/// The kinds of errors that can occur when checking if a rust enum matches a database enum or table.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ErrorKind {
  MissingFromDb(Vec<String>),
  MissingFromRustEnum(Vec<String>),
  IdMismatches(Vec<(String, i64, i64)>),
}

/// An error that is produced when a rust enum does not match a database enum or table.
///
/// It includes the list of errors that may occur simultaneously, such as id mismatches as well as missing variants.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Error)]
pub struct DbEnumError {
  pub rust_enum: String,
  pub db_source: DbEnumSource,
  pub errors: Vec<ErrorKind>,
}

impl DbEnumError {
  /// Creates a new error. Usually it's not necessary to use this directly.
  pub fn new(rust_enum: String, db_source: DbEnumSource) -> Self {
    Self {
      rust_enum,
      db_source,
      errors: Vec::new(),
    }
  }
}

/// The database source for an enum mapping. It can be the name of a custom type (for postgres) or a regular column in other databases.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DbEnumSource {
  CustomEnum(String),
  Column { table: String, column: String },
}

impl DbEnumSource {
  /// The name of the target source (a custom postgres type or a regular column).
  pub fn name(&self) -> String {
    match self {
      Self::CustomEnum(name) => name.clone(),
      Self::Column { table, column } => format!("{table}.{column}"),
    }
  }

  /// The type of the target source (a postgres enum or regular column)
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
