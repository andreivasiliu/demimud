mod acting; // Process and output things like "$n flexes $s muscles."
mod agent; // Object providing access to all game resources needed for commands
mod colors; // Turn codes like "`w" into "\e[37m".
mod commands; // do_say, do_look, do_get, etc, implemented upon EntityAgent
mod components; // Types of game data (mob, obj, etc) attached to entities
mod entity; // Every object in the world and relation between objects
mod file_parser; // Dawn of Time area format parser primitives
mod files; // Abstraction trait for reading files
mod find_entities; // Primitives to help with matching and filtering entities
mod import; // Use templates from a DoT world to insert new EntityWorld entities
mod load; // Dawn of Time area loader
mod mapper; // Map generator
mod mobprogs; // MobProg script runner, and additional do_mob_... commands
#[cfg(feature = "net")]
mod net; // Handle network players from NetServer; not used in WASM or CLI.
mod socials; // Load socials from socials.txt
mod state; // Main game object, glues everything together
mod tick; // Things that mobs do every second (e.g. wandering around rooms)
mod world; // Read-only representation of a set of Dawn of Time areas

pub use colors::colorize;
pub use files::Files;
pub use state::WorldState;
