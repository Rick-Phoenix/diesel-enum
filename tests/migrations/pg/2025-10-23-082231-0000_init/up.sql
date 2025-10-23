-- Your SQL goes here

create type pokemon_type as enum (
'grass',
'poison',
'fire',
'flying',
'water',
'bug',
'normal',
'electric',
'ground',
'fairy',
'fighting',
'psychic',
'rock',
'steel',
'ice',
'ghost',
'dragon',
'dark'
) ;

create table pokemon_table (
name text primary key,
type pokemon_type not null
) ;
