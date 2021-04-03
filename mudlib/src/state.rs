//! Main game object, glues everything together.
//!
//! The `WorldState` objects holds all information about the game, except for
//! connection/socket information.
//!
//! It has two important fields:
//! * The EntityWorld, which contains all things in the game
//! * The Players struct, which is used to store output to players
//!
//! Splitting the two makes it possible to hold multiple read-only references
//! into the entity world, while mutating Players to echo things to players.
//!
//! On a crash or restart, this entire state is thrown away and reloaded.

use crate::commands::EntityAgent;
use crate::{
    acting::{PlayerEcho, Players},
    commands::update_entity_world,
    echo,
    entity::EntityWorld,
    import::{import_from_world, VnumTemplates},
    socials::Socials,
    world::World,
    Files,
};
pub struct WorldState {
    pub(crate) socials: Socials,
    pub(crate) entity_world: EntityWorld,
    pub(crate) vnum_templates: VnumTemplates,

    pub(crate) players: Players,
    pub(crate) wander_ticks: u8,
}

pub(super) fn create_state(world: World, socials: Socials) -> WorldState {
    let players = Players {
        player_echoes: Default::default(),
    };

    let mut entity_world = EntityWorld::new();
    let vnum_templates = import_from_world(&mut entity_world, &world);

    WorldState {
        entity_world,
        vnum_templates,
        socials,
        players,
        wander_ticks: 0,
    }
}

impl WorldState {
    pub fn from_files(files: &dyn Files) -> WorldState {
        let world = crate::world::load_world(files, "data/area");
        let socials = crate::socials::load_socials(files, "data/socials.txt");
        create_state(world, socials)
    }

    pub fn update_world(&mut self) {
        update_entity_world(self);
    }

    pub fn add_player(&mut self, name: &str) {
        let player_components = self.entity_world.make_player_components(name);

        let player_id = self.entity_world.add_player(name, player_components);
        let starting_location = self
            .entity_world
            .landmark("gnomehill")
            .expect("Starting location should exist");
        self.entity_world.move_entity(player_id, starting_location);
        let mut agent = EntityAgent::new(self, player_id);
        agent.add_silver(200, player_id);

        self.players
            .player_echoes
            .insert(name.to_string(), PlayerEcho::default());

        let player = self.entity_world.entity_info(player_id);
        let mut act = self.players.act_alone(&player);
        echo!(act.others(), "$^$n materializes from thin air.\r\n");
    }

    pub fn process_player_command(&mut self, player: &str, words: &[&str]) {
        crate::commands::process_player_command(self, player, words);
    }

    pub fn player_echoes(&mut self, player: &str) -> Option<&mut String> {
        self.players
            .player_echoes
            .get_mut(player)
            .map(|echoes| &mut echoes.echo_buffer)
    }
}
