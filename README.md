# Enums For Diesel + Sqlite

This crate uses a proc macro to allow the usage of rust enums as text in sqlite with diesel.

It can be used purely for consistency during read/write operations, or it can be mapped to values belonging to a column in a table.

## Usage

Simply use the `#[diesel_enum]` on an enum.

Example:

```rust
#[derive(Debug)]
#[diesel_sqlite_enum(conn = crate::establish_connection)]
pub enum PokemonTypes {
  Grass,
  Poison,
  Fire,
  Flying,
  Water,
  Bug,
  Normal,
  Electric,
  Ground,
  Fairy,
  Fighting,
  Psychic,
  Rock,
  Steel,
  Ice,
  Ghost,
  Dragon,
  Dark,
}
```

The macro will take care of implementing `FromSql`, `FromSqlRow`, `ToSql` and `AsExpression` for the target enum.

### Automatic tests for consistency

Unless the feature `default-skip-check` is enabled or `skip_check` is used on an attribute, this crate will automatically generate a test that will connect to the database and will check if the enum variants in the rust enum correspond to the values in the target column.

If values are supposed to be a strict mapping, this is recommended as it prevents from building an app that is out of sync with the database.

### Attributes

Here are the supported attributes:

- **conn**
    Example: `(conn = crate::establish_connection)`

    It must be a path to a function that returns a database connection. It is used inside the generated tests to connect to the database.

    Since the function will be used inside a module created with `#[cfg(test)]`, the function may falsely appear like an unused import. 
    
    For that reason it is advised to use a full path.
- **skip_check**
    Example: `(skip_check)`

    Skips the generation of consistency checks
- **table**
    Example: `(table = "types")`

    By default, this crate assumes that the target table corresponds to the name of the enum in snake case. So for the example above, it would look for a column called `name` in a table called `pokemon_types`. This can be overridden with this attribute.
- **column**
    Example: `(column = "id")`

    By default, the crate looks for the values in a column called `name`. This can be used to override that behaviour.
- **case**
    Example: `(case = "snake_case")`

    By default, the variant names and the values in the table are expected to be the same. So for the example above, the values for `pokemon_types.name` would need to be `Fire`, `Grass` and so on. This can be overridden with this attribute.

    Allowed values are: [ snake_case, UPPER_SNAKE, kebab-case, camelCase, PascalCase, lowercase, UPPERCASE ]
