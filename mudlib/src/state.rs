use crate::{
    acting::{PlayerEcho, Players},
    commands::update_entity_world,
    echo,
    entity::EntityWorld,
    socials::Socials,
    world::World,
};

pub(super) struct WorldState {
    pub(crate) socials: Socials,
    pub(crate) entity_world: EntityWorld,

    pub(crate) players: Players,
}

pub(super) fn create_state(world: World, socials: Socials) -> WorldState {
    let players = Players {
        player_echoes: Default::default(),
    };

    let entity_world = EntityWorld::from_world(&world);

    WorldState {
        entity_world,
        socials,
        players,
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

        self.players
            .player_echoes
            .insert(name.to_string(), PlayerEcho::default());

        let player = self.entity_world.entity_info(player_id);
        let mut act = self.players.act_alone(&player);
        echo!(act.others(), "$^$n materializes from thin air.\r\n");
    }
}
