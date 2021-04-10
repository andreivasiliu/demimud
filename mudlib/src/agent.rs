use crate::{WorldState, acting::{InfoTarget, Players}, components::{Components, EntityType, GeneralData, Silver}, echo, entity::{EntityId, EntityWorld}, find_entities::MatchError, import::VnumTemplates, mobprogs::Action, socials::Socials, state::Area, world::{Gender, Vnum, opposite_direction}};
use crate::components::InternComponent;

pub(crate) struct EntityAgent<'e, 'p> {
    pub entity_world: &'e mut EntityWorld,
    pub socials: &'e Socials,
    pub vnum_templates: &'e VnumTemplates,
    pub areas: &'e Vec<Area>,
    pub players: &'p mut Players,

    pub entity_id: EntityId,
}

impl EntityAgent<'_, '_> {
    pub fn new<'a>(world_state: &'a mut WorldState, entity_id: EntityId) -> EntityAgent<'a, 'a> {
        EntityAgent {
            entity_world: &mut world_state.entity_world,
            socials: &world_state.socials,
            vnum_templates: &world_state.vnum_templates,
            areas: &world_state.areas,
            players: &mut world_state.players,

            entity_id,
        }
    }

    pub fn switch_agent<'a>(&'a mut self, entity_id: EntityId) -> EntityAgent<'a, 'a> {
        EntityAgent {
            entity_world: self.entity_world,
            socials: self.socials,
            vnum_templates: self.vnum_templates,
            areas: self.areas,
            players: self.players,

            entity_id,
        }
    }
}

impl<'e, 'p> EntityAgent<'e, 'p> {
    pub fn info(&mut self) -> InfoTarget<'_> {
        let myself = self.entity_world.entity_info(self.entity_id);
        self.players.info(&myself)
    }

    pub fn echo_error(&mut self, error: MatchError) {
        let myself = self.entity_world.entity_info(self.entity_id);
        match error {
            MatchError::Message(error) => {
                echo!(self.info(), "{}\r\n", error);
            }
            MatchError::MessageWithActor(error, target_id) => {
                let target = self.entity_world.entity_info(target_id);
                let mut act = self.players.act_with(&myself, &target);
                echo!(act.myself(), "{}\r\n", error);
            }
        }
    }

    pub fn check_followers(&mut self, from_room_id: EntityId, direction: &str, to_room_id: EntityId) {
        let mut followers = Vec::new();
        let myself = self.entity_world.entity_info(self.entity_id);
        let room = self.entity_world.entity_info(from_room_id);

        for follower in room.contained_entities() {
            let main_keyword = myself.main_keyword();
            if let Some(following) = &follower.components().general.following {
                if following == main_keyword {
                    followers.push(follower.entity_id());

                    let mut act = self.players.act_with(&follower, &myself);
                    echo!(act.myself(), "You follow $N to the {}.\r\n", direction);
                    echo!(act.others(), "$^$n follows $N to the {}.\r\n", direction);
                }
            }
        }

        for follower_id in followers {
            let myself = self.entity_world.entity_info(self.entity_id);
            let follower = self.entity_world.entity_info(follower_id);
            let mut act = self.players.act_with(&follower, &myself);
            echo!(
                act.target(),
                "$^$n follows you in from the {}.\r\n",
                opposite_direction(direction)
            );
            echo!(
                act.others(),
                "$^$n follows $N in from the {}.\r\n",
                opposite_direction(direction)
            );

            self.entity_world.move_entity(follower_id, to_room_id);
            let mut agent = self.switch_agent(follower_id);
            agent.do_look();
            agent.check_triggers_self(Action::Entry);
        }
    }
}

// Silver handling
impl<'e, 'p> EntityAgent<'e, 'p> {
    pub fn add_silver(&mut self, amount: usize, to_entity: EntityId) {
        let entity = self.entity_world.entity_info(to_entity);

        // See if the silver coins can be joined with an existing pile in the inventory
        let silver_pile = entity
            .objects()
            .find(|object| object.components().silver.is_some())
            .map(|object| object.entity_id());

        match silver_pile {
            Some(silver_pile_id) => {
                // Add to an existing pile
                let mut silver_pile = self.entity_world.entity_info_mut(silver_pile_id);

                let silver = silver_pile
                    .components()
                    .silver
                    .as_mut()
                    .expect("Filtered above to one that has the component");

                silver.amount += amount;
                let new_amount = silver.amount;

                let (mut silver_pile, interner) = self
                    .entity_world
                    .entity_info_mut_with_interner(silver_pile_id);

                let short_description = format!("{} silver coins", new_amount);
                silver_pile.set_short_description(interner, &short_description);
            }
            None => {
                // Otherwise create a new pile
                let keyword = "silver coins";
                let short_description = format!("{} silver coins", amount);

                let title = "Swimming in silver coins.";
                let internal = "You are inside a pile of silver coins.";
                let external = "A pile of silver coins.";
                let lateral = "A pile of silver coins is on the ground here.";

                let act_info = self.entity_world.interner.act_info(
                    keyword,
                    &short_description,
                    Gender::Neutral,
                );
                let descriptions = self
                    .entity_world
                    .interner
                    .descriptions(title, internal, external, lateral);

                self.entity_world.insert_entity(
                    to_entity,
                    Components {
                        act_info,
                        descriptions,
                        general: GeneralData {
                            vnum: Vnum(0),
                            area: "silver".to_string(),
                            sector: None,
                            entity_type: EntityType::Object,
                            equipped: None,
                            command_queue: Vec::new(),
                            following: None,
                        },
                        mobile: None,
                        object: None,
                        door: None,
                        mobprog: None,
                        silver: Some(Silver { amount }),
                    },
                );
            }
        };
    }

    pub fn remove_silver(&mut self, amount: usize, to_entity: EntityId) -> bool {
        let entity = self.entity_world.entity_info(to_entity);

        // See if the silver coins can be joined with an existing pile in the inventory
        let silver_pile = entity
            .objects()
            .find(|object| object.components().silver.is_some())
            .map(|object| object.entity_id());

        match silver_pile {
            Some(silver_pile_id) => {
                let new_amount = {
                    let mut silver_pile = self.entity_world.entity_info_mut(silver_pile_id);
                    let silver = silver_pile
                        .components()
                        .silver
                        .as_mut()
                        .expect("Filtered above to one that has the component");

                    if silver.amount >= amount {
                        silver.amount -= amount;
                        Some(silver.amount)
                    } else {
                        None
                    }
                };

                if let Some(new_amount) = new_amount {
                    if new_amount != 0 {
                        let (mut silver_pile, interner) = self
                            .entity_world
                            .entity_info_mut_with_interner(silver_pile_id);

                        let short_description = format!("{} silver coins", new_amount);
                        silver_pile.set_short_description(interner, &short_description);
                    } else {
                        let mut agent = self.switch_agent(silver_pile_id);
                        agent.do_die();
                    }
                }
                new_amount.is_some()
            }
            None => false,
        }
    }
}
