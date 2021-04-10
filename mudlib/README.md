# MudLib

This is the main MUD logic.

It is compiled as a shared object (.so/.dll/.dylib), loaded dynamically by `netcore`, and can be reloaded to hot-swap code without dropping network connections.

Running `cargo watch -x build` in this directory can make hot-swapping code a simple matter of typing `restart` inside the MUD.

# Files

* net - Handle network events from NetServer; not used in WASM or CLI.
  * Provides the main loop; may end the loop and ask `netcore` to unload and reload the module
  * Uses `NetServer` from this cargo workspace's `netcore` crate
  * Manages `Connections` and forwards commands from sockets to game entities
  * The `Connections` object is serialized and sent to the next instance when hot-swapping code
* acting - Process and output things like "$n flexes $s muscles."
  * Provides `.act_alone(&myself)` and `.act_with(&myself, &other)` on `agent.players`
  * Main method of sending text to the player, the target, and others in the room
* colors - Turn codes like "`w" into "\e[37m".
  * Provides a `colorize(text)` method
* agent - Object providing access to all game resources needed for commands
  * Provides `EntityAgent`, upon which all player/mob commands are implemented
  * Provides mutable references to the world state and echo buffers, and read-only access to data
  * The `EntityAgent` supports only one of these at a time: mutating one entity, or examining many entities
  * Has an `EntityId` referring to whoever is doing the command
* commands - do_say, do_look, do_get, etc
  * Most commands that players and entities can do are here
  * The commands are on the `agent::EntityAgent` object, which gives access to all game resources
* mobprogs - MobProg script runner, and additional do_mob_... commands
  * Provides the do_mob command, which has several mob-specific subcommands
  * Can check triggers for actions that happen in the room and run associated mobprogs
  * Can read mobprog code and execute it line by line to make mobs do things
* tick - Things that mobs do every second (e.g. wandering around rooms)
  * Has `update_wander()`, which makes mobs move aroud a bit every 4 seconds
  * Has `update_command_queue()`, which runs commands that were queued with a delay
* components - Types of game data (mob, obj, etc) attached to entities
  * Components for entities (objects, mobs, rooms, etc) which hold state for that entity
* entity - Every object in the world and relation between objects
  * Provides the `EntityWorld`, the place where the entire game state lives in
  * Provides an `EntityInfo<'_>` to examine entities, with many helper methods on it
  * Provides a short-lived `EntityId` to refer to entities without a reference
  * Provides a long-lived `PermanentId` which is not guaranteed to be alive anymore
* find_entities - Primitives to help with matching and filtering entities
  * Provides an `EntityIterator`, with various methods to filter them
  * This is the main way of finding mobs/objects in the same room, in the inventory, etc
  * The entities are turned into `MatchCandidate` objects with information about if/why they were rejected
* files - Abstraction trait for reading files
  * Can either use the filesystem normally, or embeds area files if compiled to WASI
* file_parser - Dawn of Time area format parser primitives
  * Provides `FileParser` with helper methods to parse DoT files
  * Has methods like `.read_until_newline`, `.read_until_tilde()`, `.skip_one_space()`
* load - Dawn of Time area loader
  * Looks at an `.are` file and loads all rooms, mobs, objects, mobprogs, resets, and shops
  * Constructs an `Area` object representing all rooms/mobs/etc in that area
  * The mobs and objects here are just templates
* world - Read-only representation of a set of Dawn of Time areas
  * Merges `Area` objects loaded from all files in the `data/area` directory
  * Only holds templates, not state
* import - Convert a DoT world to EntityWorld entities
  * Takes a read-only `World` object, and spawns entities for each room, mobile, object
  * Rooms are spawned immediately; objects/mobs are stored in a vnum-to-template map
  * The Area's reset commands are used to spawn multiple mobs/objects of a single mob/object template
* mapper - Map generator
  * Generates a colored ASCII map for the `map` command
  * Recursivelty scans the rooms starting from the current player's room
* socials - Load socials (aka emotes) from socials.txt
  * Provides a `Socials` object that has a lot of `Social` objects
  * Each social has messages for targetted, untargetted, and self-targetted
* state - Main game object, glues everything together
  * A small object that holds the `EntityWorld`, the `Players`, and the `Socials`.
  * Provide the `WorldState`, which can forward commands to entities and returns things to echo
