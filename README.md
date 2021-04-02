# DemiMUD

DemiMUD is a [MUD](https://en.wikipedia.org/wiki/MUD) codebase written in
[Rust](https://www.rust-lang.org/).

It's mostly made just for fun, and to learn Rust, with no concrete plans to run
it somewhere. Given its features however, it may be a very good source of
inspiration for other aspiring MUD coders.

# Features

## Status

The MUD only has about 10 commands (`say`, `look`, `get`, `drop`, `map`,
movement commands and emotes).

It is able to load some basic room and mob descriptions from the Dawn of Time
([repo](https://github.com/mudhistoricalsociety/dawnoftime_1.69r)) stock areas,
and allow exploration, but no interaction with anything except for picking up
and dropping objects.

## Hot-swapping

Because DemiMUD is a way to learn coding, DemiMUD is specifically built for
development.

DemiMUD is split in two crates: `netcore` and `mudlib`, with `mudlib` compiling
has a shared object (.dll/.so/.dylib). This split allows `netcore` to keep up
connections alive while `mudlib` is unloaded and reloaded again, in order to
hot-swap code.

`mudlib` also uses `catch_unwind()` in a couple of places and reloads the whole
world on panics while throwing away the old (likely corrupted) world, which
makes it convenient while coding new features. With something like
`cargo watch`, simply saving a file and sending the `restart` command is
enough to get the new code up and running.

## Everything is an entity

Frustrated by the limitation of many MUDs to restrict using skills or abilities
on certain types of objects (e.g. only on players, or only on mobs), I decided
to make everything be an entity.

### Everything is a room

Rooms, objects, mobs, players, are all rooms.

The MUD is built to model all of its entities as containers, which allows for
more natural handling of e.g. the insides of vehicles, picking up pets,
allowing things to happen inside a player's inventory, etc.

### Everything is an object

Rooms and mobs are objects, and so are their exits. Someone can pick up a room,
or a mob, or even pick up an exit. Don't worry, that mob is not stuck there, it
can exit you by going through the exit you just picked up.

### Entities have components

DemiMUD is not based on an Entity Component System, but it takes some concepts
from it.

Rather than segregating things into types, things instead have capabilities. An
object can be a weapon, have health, and be the target of a fireball, all at
the same time.

The capability to contain and be contained, to be adressed (short description
and gender) and to be described (internal/external descriptions) are universal.

# Running

Currently only tested on Windows; there might be issues with CRLF line endings
on other systems.

To build it, `git clone` this repository and run `cargo build --release`. You
will need the Rust compiler (see https://rustup.rs) and any of its dependencies
(e.g. MSVC build tools on Windows).

The game currently uses area data from Dawn of Time and socials from
Ultra-Envy, see the README.md file inside `./data/` on how to get them.

To run it, run `cargo run --release` or run the `target/release/netcore`
executable directly; `netcore` will then load `target/release/mudlib.dll` (or
`.so` or `.dylib` on Linux/MacOS) from the binary's directory.
