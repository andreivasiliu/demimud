use crate::{acting::{PlayerEcho, Players}, commands::update_entity_world, echo, entity::EntityWorld, import::{VnumTemplates, import_from_world}, socials::Socials, world::World};
use crate::{
    commands::EntityAgent,
};
pub(super) struct WorldState {
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
    pub(super) fn update_world(&mut self) {
        update_entity_world(self);
    }

    pub(super) fn add_player(&mut self, name: &str) {
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
}
