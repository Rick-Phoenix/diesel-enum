// @generated automatically by Diesel CLI.

diesel::table! {
    pokemon_types (pokemon_id, type_id) {
        pokemon_id -> Integer,
        type_id -> Integer,
    }
}

diesel::table! {
    pokemons (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    types (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::joinable!(pokemon_types -> pokemons (pokemon_id));
diesel::joinable!(pokemon_types -> types (type_id));

diesel::allow_tables_to_appear_in_same_query!(pokemon_types, pokemons, types,);
