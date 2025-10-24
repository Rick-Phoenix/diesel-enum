# How It Works

This crate allows for seamless mapping of rust enums to database enums or lookup tables using [`diesel`](::diesel), and automatically generates methods and tests that connect to the database to ensure that the rust enum is fully in sync with the database source.

The mappings can be done with two kinds of sources:

- For **custom types** created in **Postgres**, it maps the rust enum to the custom type.

    In this case, `diesel.toml` must be configured like this:
    ```toml
    custom_type_derives = ["diesel::query_builder::QueryId"]
    ```
    and then the enum variants will simply be serialized/deserialized as the corresponding member of the postgres enum.
    
- For other databases such as **SQLite** and **MySQL**, a regular lookup table is used. 

    For these, you can choose between two kinds of mappings: **id mappings** and **name mappings**, which can be used together or in isolation.

Let's look at an example to better illustrate the process.

## Id Mapping

Let's say that we have a database table called `pokemon_types` with these values

```text
    id      name
-------------------------
     1      Fire
     2      Grass
     3      Water
```

and a corresponding rust enum and struct

```rust,ignore
pub enum PokemonType {
  Fire,
  Grass,
  Water
}

pub struct Pokemon {
  pub name: String,
  pub type_id: i32
}
```

When using an **id mapping**, the macro will generate [`Into`]/[`TryFrom`] implementations with the target integer value (i.e. `i32` for `Integer` and so on), as well as `FromSql` and `ToSql` implementations that will use the id belonging to each variant when deserializing/serializing the enum's value.

This means that we can effectively replace the `type_id` field with the enum, so that it will behave exactly like an id but with a bit more clarity and ease of use

```rust,ignore
pub struct Pokemon {
  pub name: String,
  pub type_id: PokemonType
}
```

### Managing ID Mapping

The [`Into`]/[`TryFrom`] implementations with the integer values will be based on the order of the variants, assuming an auto-incrementing integer is used in the database.

While this can be overridden for single variants, things can become unwieldy if a variant in the middle is deleted, as all the following ids will now need to be set manually.

```rust,ignore
pub enum PokemonType {
  // Removed from the database
  Grass,
  // Removed from the database
  // Poison,
  // Now all these would require manual mapping
  #[db_mapping(id = 3)] // Would be 1
  Fire,
  #[db_mapping(id = 4)] // Would be 2
  Flying,
  #[db_mapping(id = 5)] // Would be 3
  Water
}
```

For such situations, we can use the `skip_ids` parameter to list some ranges or numbers that should be skipped from the [`Into`]/[`TryFrom`] integer conversion.

```rust,ignore
// using skip_ids(1..=2) or skip_ids(1, 2)
pub enum PokemonType {
    // Grass,
    // Poison,
    // Now these will be correct
    Fire, // 3
    Flying, // 4
    Water // 5
}
```

## Text-based mappings 

Alternatively, we can also use a text-based mapping, that will instead map to the text value of the variant. So we would go from this:

```rust,ignore
pub struct Pokemon {
  pub name: String,
  pub type_: String
}
```

to this

```rust,ignore
pub struct Pokemon {
  pub name: String,
  pub type_: PokemonType
}
```

In such a case, the mapped database type will simply be `Text` (or the specific custom type, if a custom postgres type is used).

## Using Both Mappings

It is also possible to use both mappings. In such a case, the macro will treat the normal enum as one with a text-based mapping, and it will also create a copy of the same enum with an `Id` suffix, that will be mapped to the table's `id` column. So in this example, it would automatically generate the following struct:

```rust,ignore
pub enum PokemonTypeId {
  Fire,
  Grass,
  Water
}
```

It will also generate [`From`] implementations so that `PokemonType` can be **seamlessly converted** into `PokemonTypeId` and vice versa.

## Generated Consistency Checks

The macro will also generate a method called `check_consistency`, that will connect to the database and check if the mapped enum is consistent with the rust enum. If it is not, it will return a [`DbEnumError`], which will contain the source of the error such as missing variants or an `id` mismatc.

By default, it will also generate a test that will call that method and panic if it returns an error.

# Macro Attributes

These are the allowed parameters for the `#[diesel_enum(...)]` macro.

- `id_mapping`
    - `id_mapping(default)` uses a default mapping with `Integer` and `i32`.
    - `id_mapping(sql_type = diesel::sql_type::...)` can be used to customize the mapped type (only integer-based types are supported)
        - This type will directly be passed to `#[diesel(sql_type = ...)]`.
    - Ignored if `name_mapping` is used with a custom type.

- `skip_ids(1..=15, 20, 22, 30..35)`
    - Specifies a list of numbers or ranges to skip when generating conversions to/from integers for id-based mappings.

- `name_mapping`
    - `name_mapping(default)` uses the default mapping as a regular column as `Text`
    - `name_mapping(path = crate::schemas::MyCustomType)` specifies the path to a custom generated type from postgres
        - Required for custom postgres types
        - This type will directly be passed to `#[diesel(sql_type = ...)]`.
    - `name_mapping(name = "my_custom_type")` specifies the name of the custom type inside postgres. 
        - If unset, the last segment from `path` in snake_case will be used instead

- `table_name = "my_table"`
    - The table to use when mapping to a regular lookup table. Ignored for custom types. 
    - It defaults to the name of the enum in snake_case

- `table`
    - The path to the target table struct inside the generated schema from diesel.
    - If unset, it defaults to `crate::schema::$NAME`, where `$NAME` is the value from `table_name`

- `column`
    - The column to use for enums that map to regular columns.
    - Defaults to `name` (so for a `PokemonTypes` enum, the default target will be the column `pokemon_types.name`)

- `case`
    - Determines the casing of the variants in the custom type/database column.
    - Accepted values are: `[ snake_case, UPPER_SNAKE, camelCase, PascalCase, lowercase, UPPERCASE, kebab-case ]`
    - Defaults to snake_case.

- `conn`
    - The path to the test runner, namely the function that is called in the generated method and test to check the validity of the enum mapping.
    - It should receive a callback where a database connection is passed as the only argument:
        ```rust,ignore
        async fn my_runner(
          callback: impl FnOnce(&mut SqliteConnection) -> Result<(), DbEnumError> + std::marker::Send + 'static
        ) -> Result<(), DbEnumError>
        ```
    - There are some default runners exported within this crate: [`sqlite_runner`] (with the `sqlite` feature) or [`postgres_runner`] (with the `postgres` feature), that set up a connection pool with `deadpool-diesel` and run the tests with it.

- `skip_check`
    - The macro will not generate the `check_consistency` method that can be used for checking the validity of the database mapping.
    - Can be useful in case the rust enum is to be used as a simple way of enforcing a set of predetermined values, rather than a full mapping to a database structure.

- `skip_test`
    - By default, the macro will generate a test that runs the consistency check and panics if the mapping is out of sync. This parameter disables that behaviour.
    - Automatically true is `skip_check` is true.

## Variant Attributes

Variant attributes can be set with `#[db_mapping(name = "...", id = ...)]`

- `name`
    - Manually sets the corresponding name of the variant in the database source.
    - Overrides the top-level `case` parameter.

- `id`
    - Manually sets the corresponding id of the variant in the database source.
    - Ignored for postgres custom types.

# Warnings And Considerations

- Sometimes there may be some weird issues caused by the order in which the macros are expanded. 
    For this, it is advised to call the macro **before all other macros**.

- The [`diesel_enum`] macro automatically implements the following derives on the target enum:
    - [`PartialEq`], [`Eq`], [`Clone`], [`Copy`], [`Hash`], [`Debug`]
    - [`FromSqlRow`](diesel::deserialize::FromSqlRow), [`AsExpression`](diesel::expression::AsExpression), [`ToSql`](diesel::serialize::ToSql), [`FromSql`](diesel::deserialize::FromSql)
    
    And also passes the target type to the `#[diesel(sql_type = ...)]` attribute.
    
    So an error may occur if trying to set these a second time.

- When using a double mapping the `Id` enum will be a **full copy** of the original one, including all the given macro attributes. If this is an issue, then the enum may only have a name mapping, and the custom id mapping can be implemented manually.

- When using a double mapping, `check_consistency` will only be generated for the enum with the original name, but it will check the mappings for both the variant names and ids.
