#[cfg(feature = "net")]
mod net; // Handle network players from NetServer; not used in WASM or CLI.
mod acting; // Process and output things like "$n flexes $s muscles."
mod colors; // Turn codes like "`w" into "\e[37m".
mod agent; // Object providing access to all game resources needed for commands
mod commands; // do_say, do_look, do_get, etc
mod mobprogs;  // MobProg script runner, and additional do_mob_... commands
mod tick; // Things that mobs do every second (e.g. wandering around rooms)
mod components; // Types of game data (mob, obj, etc) attached to entities
mod entity; // Every object in the world and relation between objects
mod file_parser; // Dawn of Time area format parser primitives
mod import; // Convert a DoW world to EntityWorld entities
mod load; // Dawn of Time area loader
mod files; // Abstraction trait for reading files
mod mapper; // Map generator
mod socials; // Load socials from socials.txt
mod state; // Main game object, glues everything together
mod world; // Read-only representation of a set of Dawn of Time areas
mod find_entities; // Primitives to help with matching and filtering entities

pub use files::Files;
pub use state::WorldState;
pub use colors::colorize;
