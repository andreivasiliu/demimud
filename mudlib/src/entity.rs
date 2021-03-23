use std::{
    collections::{BTreeMap, HashMap},
    num::NonZeroUsize,
};

use inflector::Inflector;
use string_interner::StringInterner;

use crate::{
    components::{Components, EntityComponentInfo, EntityType, GeneralData, InternComponent},
    import::import_from_world,
    world::{Gender, World},
};

pub(crate) struct EntityWorld {
    id_generator: IdGenerator,
    // FIXME
    pub interner: StringInterner,
    entities: HashMap<RawEntityId, Entity>,
    player_entities: HashMap<String, RawEntityId>,
    player_locations: BTreeMap<String, RawEntityId>,
    landmarks: BTreeMap<&'static str, RawEntityId>,
    world_entity_id: RawEntityId,
    era: u16,
}

struct Entity {
    components: Components,
    raw_entity_id: RawEntityId,
    contents: Vec<RawEntityId>,
    contained_by: Option<RawEntityId>,
    leads_to: Option<RawEntityId>,
    leads_from: Vec<RawEntityId>,

    player: Option<String>,
}

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntityId {
    id: RawEntityId,
    era: u16,
}

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
struct RawEntityId {
    id: NonZeroUsize,
}

pub(crate) struct EntityInfo<'e> {
    entity: &'e Entity,
    entity_world: &'e EntityWorld,
}

pub(crate) struct EntityInfoMut<'e> {
    entity: &'e mut Entity,
}

pub(crate) enum Found<'a> {
    Myself,
    Other(EntityInfo<'a>),
    WrongSelf,
    WrongOther(EntityInfo<'a>),
    Nothing,
}

struct IdGenerator {
    next_entity_id: NonZeroUsize,
}

impl IdGenerator {
    fn next(&mut self) -> RawEntityId {
        let new_id = self.next_entity_id;
        let next_id = self.next_entity_id.get().wrapping_add(1);

        self.next_entity_id = match NonZeroUsize::new(next_id) {
            Some(id) => id,
            // Reminder: max is 18446744073709551615, so it's unlikely
            None => panic!("Ran out of IDs!"),
        };

        RawEntityId { id: new_id }
    }
}

impl EntityWorld {
    fn new() -> Self {
        let mut interner = StringInterner::new();
        let mut id_generator = IdGenerator {
            next_entity_id: NonZeroUsize::new(1).expect("1 != 0"),
        };

        let keyword = "world";
        let short_description = "the entire world";

        let title = "The World";
        let external = "This is the whole world. It contains everything.";
        let internal = "You are inside the world, and the entire world surrounds you.";
        let lateral = "The entire world sits calmly next to you.";

        // This is the only method to create an entity contained by no other entity.
        let world_entity = Entity {
            components: Components {
                act_info: interner.act_info(keyword, short_description, Gender::Neutral),
                descriptions: interner.descriptions(title, internal, external, lateral),
                general: GeneralData {
                    area: "world".to_string(),
                    sector: None,
                    entity_type: EntityType::Room,
                    equipped: None,
                },
                mobile: None,
            },
            raw_entity_id: id_generator.next(),
            contents: Vec::new(),
            contained_by: None,
            leads_to: None,
            leads_from: Vec::new(),
            player: None,
        };

        let world_entity_id = world_entity.raw_entity_id;

        let mut entities = HashMap::with_capacity(1);
        entities.insert(world_entity_id, world_entity);

        EntityWorld {
            id_generator,
            interner,
            entities,
            player_entities: HashMap::new(),
            player_locations: BTreeMap::new(),
            landmarks: BTreeMap::new(),
            world_entity_id,
            era: 1,
        }
    }

    pub fn move_entity(&mut self, entity_id: EntityId, to_room_id: EntityId) {
        let raw_entity_id = self.raw_entity_id(entity_id);

        // Move out of old room
        let original_room = self.entity_raw(raw_entity_id).contained_by;
        if let Some(room) = original_room {
            let room = self.entity_mut_raw(room);
            room.contents
                .retain(|contained_entity_id| contained_entity_id != &raw_entity_id)
        }

        // Move into new room
        self.entity_mut(to_room_id).contents.push(raw_entity_id);
        self.entity_mut(entity_id).contained_by = Some(self.raw_entity_id(to_room_id));

        // Update world references
        if let Some(player) = &self
            .entities
            .get(&entity_id.id)
            .expect("Checked above")
            .player
        {
            if let Some(location) = self.player_locations.get_mut(player) {
                *location = to_room_id.id;
            }
        }
    }

    pub fn make_player_components(&mut self, name: &str) -> Components {
        let keyword = name.to_lowercase();
        let short_description = name.to_title_case();
        let proper_name = name.to_title_case();
        let title = format!("Inside {}.", proper_name);
        let external = format!(
            "{} is a player. Players don't yet have a description.",
            proper_name
        );
        let internal = format!(
            "You are inside {}, the player. How did you get in here?",
            proper_name
        );
        let lateral = format!("{}, a player, is here.", proper_name);

        Components {
            act_info: self
                .interner
                .act_info(&keyword, &short_description, Gender::Male),
            descriptions: self
                .interner
                .descriptions(&title, &internal, &external, &lateral),
            general: GeneralData {
                area: "players".to_string(),
                sector: None,
                entity_type: EntityType::Player,
                equipped: None,
            },
            mobile: None,
        }
    }

    pub fn add_player(&mut self, name: &str, components: Components) -> EntityId {
        if let Some(player_entity_id) = self.player_entities.get(name) {
            return EntityId {
                id: *player_entity_id,
                era: self.era,
            };
        }

        let room_entity_id = EntityId {
            id: self.world_entity_id,
            era: self.era,
        };

        let player_entity_id = self.insert_entity(room_entity_id, components);

        self.entity_mut(player_entity_id).player = Some(name.to_string());

        self.player_entities
            .insert(name.to_string(), player_entity_id.id);
        self.player_locations
            .insert(name.to_string(), room_entity_id.id);

        player_entity_id
    }

    fn raw_entity_id(&self, entity_id: EntityId) -> RawEntityId {
        if entity_id.era != self.era {
            panic!("Entity IDs should not be stored long-term!");
        }
        entity_id.id
    }

    fn entity(&self, entity_id: EntityId) -> &Entity {
        self.entity_raw(self.raw_entity_id(entity_id))
    }

    fn entity_raw(&self, raw_entity_id: RawEntityId) -> &Entity {
        self.entities
            .get(&raw_entity_id)
            .expect("Entities should not be deleted within an era")
    }

    fn entity_mut(&mut self, entity_id: EntityId) -> &mut Entity {
        self.entity_mut_raw(self.raw_entity_id(entity_id))
    }

    fn entity_mut_raw(&mut self, raw_entity_id: RawEntityId) -> &mut Entity {
        self.entities
            .get_mut(&raw_entity_id)
            .expect("Entities should not be deleted within an era")
    }

    fn entity_info_raw(&self, entity_id: RawEntityId) -> EntityInfo<'_> {
        let entity = self
            .entities
            .get(&entity_id)
            .expect("Internally constructed IDs should be correct");

        EntityInfo {
            entity: &entity,
            entity_world: &self,
        }
    }

    pub fn entity_info(&self, entity_id: EntityId) -> EntityInfo<'_> {
        let entity = self.entity(entity_id);

        EntityInfo {
            entity,
            entity_world: self,
        }
    }

    pub fn entity_info_mut(&mut self, entity_id: EntityId) -> EntityInfoMut<'_> {
        let entity = self.entity_mut(entity_id);

        EntityInfoMut { entity }
    }

    pub fn all_entities(&self) -> impl Iterator<Item = EntityInfo<'_>> {
        self.entities.values().map(move |entity| EntityInfo {
            entity,
            entity_world: self,
        })
    }

    pub fn add_landmark(&mut self, landmark: &'static str, entity_id: EntityId) {
        self.landmarks
            .insert(landmark, self.raw_entity_id(entity_id));
    }

    pub fn landmark(&self, landmark: &str) -> Option<EntityId> {
        self.landmarks.get(landmark).map(|room_id| EntityId {
            id: *room_id,
            era: self.era,
        })
    }

    pub fn room_of(&self, entity_id: EntityId) -> EntityId {
        let entity = self.entity(entity_id);
        let room = self
            .entities
            .get(
                &entity
                    .contained_by
                    .expect("Should never pass world's border"),
            )
            .expect("EntityWorld should manage .contained_by correctness");

        EntityId {
            id: room.raw_entity_id,
            era: self.era,
        }
    }

    pub fn player_entity_id(&self, name: &str) -> Option<EntityId> {
        self.player_entities
            .get(name)
            .map(|raw_entity_id| EntityId {
                id: *raw_entity_id,
                era: self.era,
            })
    }

    pub fn world_entity_id(&self) -> EntityId {
        EntityId {
            id: self.world_entity_id,
            era: self.era,
        }
    }

    pub fn insert_entity(&mut self, container: EntityId, components: Components) -> EntityId {
        let container = self.raw_entity_id(container);

        let raw_entity_id = self.id_generator.next();

        let new_entity = Entity {
            components,
            raw_entity_id,
            contents: Vec::new(),
            contained_by: Some(container),
            leads_to: None,
            leads_from: Vec::new(),
            player: None,
        };

        self.entities.insert(raw_entity_id, new_entity);

        let container_entity = self.entity_mut_raw(container);
        container_entity.contents.push(raw_entity_id);

        EntityId {
            id: raw_entity_id,
            era: self.era,
        }
    }

    pub(crate) fn set_leads_to(&mut self, exit_id: EntityId, to_room_id: EntityId) {
        let exit_id = self.raw_entity_id(exit_id);
        let to_room_id = self.raw_entity_id(to_room_id);

        let exit = self.entity_raw(exit_id);

        if exit.leads_to.is_some() {
            // FIXME
            unimplemented!("Changing existing exit points is not yet implemented.");
        }

        self.entity_mut_raw(exit_id).leads_to = Some(to_room_id);
        self.entity_mut_raw(to_room_id).leads_from.push(exit_id);
    }

    pub(crate) fn from_world(world: &World) -> EntityWorld {
        let mut entity_world = EntityWorld::new();

        import_from_world(&mut entity_world, world);

        entity_world
    }
}

impl<'e> EntityInfoMut<'e> {
    pub fn components(&'e mut self) -> &'e mut Components {
        &mut self.entity.components
    }
}

impl<'e> EntityInfo<'e> {
    pub fn entity_id(&self) -> EntityId {
        EntityId {
            id: self.entity.raw_entity_id,
            era: self.entity_world.era,
        }
    }

    pub fn components(&self) -> &'e Components {
        &self.entity.components
    }

    pub fn component_info(&self) -> EntityComponentInfo<'e, 'e> {
        EntityComponentInfo::new(self.components(), &self.entity_world.interner)
    }

    pub fn equipped(&self) -> Option<&str> {
        None
        // FIXME
        // self.entity
        //     .data
        //     .equipped
        //     .as_ref()
        //     .map(|location| self.resolve(location))
    }

    pub fn leads_to(&self) -> Option<EntityId> {
        self.entity.leads_to.map(|leads_to| {
            let to_room = self.entity_world.entity_raw(leads_to);
            EntityId {
                id: to_room.raw_entity_id,
                era: self.entity_world.era,
            }
        })
    }

    pub fn is_exit(&self) -> bool {
        matches!(self.entity.components.general.entity_type, EntityType::Exit)
    }

    pub fn is_extra_description(&self) -> bool {
        matches!(
            self.entity.components.general.entity_type,
            EntityType::ExtraDescription
        )
    }

    pub fn is_object(&self) -> bool {
        matches!(
            self.entity.components.general.entity_type,
            EntityType::Object
        )
    }

    pub fn is_player(&self, player_name: &str) -> bool {
        self.entity_world.player_entities.get(player_name) == Some(&self.entity.raw_entity_id)
    }

    pub fn colocated_with_player(&self, player_name: &str) -> bool {
        if let Some(room) = self.entity.contained_by {
            Some(&room) == self.entity_world.player_locations.get(player_name)
        } else {
            false
        }
    }

    fn iter_by_type(&self, entity_type: EntityType) -> impl Iterator<Item = EntityInfo<'e>> {
        let entity_world = self.entity_world;

        self.entity.contents.iter().filter_map(move |entity_id| {
            let entity = &entity_world.entities[entity_id];
            if entity.components.general.entity_type == entity_type {
                Some(EntityInfo {
                    entity,
                    entity_world,
                })
            } else {
                None
            }
        })
    }

    pub fn contained_entities(&self) -> impl Iterator<Item = EntityInfo<'e>> {
        let entity_world = self.entity_world;

        self.entity
            .contents
            .iter()
            .map(move |entity_id| entity_world.entity_info_raw(*entity_id))
    }

    pub fn contained_entities_with_descriptions(&self) -> impl Iterator<Item = EntityInfo<'e>> {
        let entity_world = self.entity_world;

        self.entity.contents.iter().flat_map(move |entity_id| {
            let entity = entity_world.entity_info_raw(*entity_id);

            entity.extra_descriptions().chain(Some(entity))
        })
    }

    pub fn exits(&self) -> impl Iterator<Item = EntityInfo<'e>> {
        self.iter_by_type(EntityType::Exit)
    }

    pub fn objects(&self) -> impl Iterator<Item = EntityInfo<'e>> {
        self.iter_by_type(EntityType::Object)
    }

    pub fn mobiles(&self) -> impl Iterator<Item = EntityInfo<'e>> {
        self.iter_by_type(EntityType::Mobile)
    }

    pub fn players(&self) -> impl Iterator<Item = EntityInfo<'e>> {
        self.iter_by_type(EntityType::Player)
    }

    pub fn extra_descriptions(&self) -> impl Iterator<Item = EntityInfo<'e>> {
        self.iter_by_type(EntityType::ExtraDescription)
    }

    pub fn find_entity<F>(&self, keyword: &str, matcher: F) -> Found<'e>
    where
        F: Fn(&EntityInfo) -> bool,
    {
        let room_id = self.entity_world.room_of(self.entity_id());
        let room = self.entity_world.entity_info(room_id);

        let mut bad_result = None;

        if ["me", "self", "myself"].contains(&keyword) {
            if matcher(self) {
                return Found::Myself;
            } else {
                bad_result = Some(self.entity_world.entity_info_raw(self.entity_id().id));
            }
        }

        let inventory_and_room = self
            .contained_entities_with_descriptions()
            .chain(room.contained_entities_with_descriptions());

        for entity in inventory_and_room {
            if entity
                .component_info()
                .keyword()
                .split_whitespace()
                .any(|word| word.eq_ignore_ascii_case(keyword))
            {
                if matcher(&entity) {
                    return if entity.entity_id() == self.entity_id() {
                        Found::Myself
                    } else {
                        Found::Other(entity)
                    };
                } else {
                    bad_result = Some(entity);
                }
            }
        }

        match bad_result {
            Some(entity) if entity.entity_id() == self.entity_id() => Found::WrongSelf,
            Some(entity) => Found::WrongOther(entity),
            None => Found::Nothing,
        }
    }
}
