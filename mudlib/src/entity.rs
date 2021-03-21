use std::{collections::{BTreeMap, HashMap}, num::NonZeroUsize};

use inflector::Inflector;
use string_interner::StringInterner;

use crate::world::{Gender, Object, ResetCommand, World};

pub(crate) struct EntityWorld {
    id_generator: IdGenerator,
    interner: StringInterner,
    entities: HashMap<RawEntityId, Entity>,
    player_entities: HashMap<String, RawEntityId>,
    player_locations: BTreeMap<String, RawEntityId>,
    starting_room: RawEntityId,
    world_entity_id: RawEntityId,
    era: u16,
}

struct IdGenerator {
    next_entity_id: NonZeroUsize,
}

impl IdGenerator {
    fn next(&mut self) -> RawEntityId {
        let new_id = self.next_entity_id;
        let next_id = self.next_entity_id.get() + 1;

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

        let mut intern = |string: &str| IntStr {
            symbol: interner.get_or_intern(string),
        };

        // This is the only method to create an entity contained by None.
        let world_entity = Entity {
            data: EntityData {
                act_info: ActInfo {
                    keyword: intern("world"),
                    short_description: intern("the entire world"),
                    gender: Gender::Neutral,
                },
                internal_title: intern("The World"),
                external_description: intern("This is the whole world. It contains everything."),
                internal_description: intern(
                    "You are inside the world, and the entire world surrounds you.",
                ),
                lateral_description: intern("The entire world sits calmly next to you."),
                entity_type: EntityType::Room,
                equipped: None,
            },
            raw_entity_id: id_generator.next(),
            contents: Vec::new(),
            contained_by: None,
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
            starting_room: world_entity_id,
            world_entity_id,
            era: 1,
        }
    }

    pub fn add_player(&mut self, name: &str) {
        let keyword = self.intern(name);
        let short_description = self.intern(&name.to_title_case());
        let title = name.to_title_case();
        let internal_title = self.intern(&format!("Inside {}.", title));
        let external_description = self.intern(&format!(
            "{} is a player. Players don't yet have a description.",
            title
        ));
        let internal_description = self.intern(&format!(
            "You are inside {}, the player. How did you get in here?",
            title
        ));
        let lateral_description = self.intern(&format!("{}, a player, is here.", title));

        let room_entity_id = EntityId {
            id: self.starting_room,
            era: self.era,
            entity_type: EntityType::Room,
        };

        let player_entity_id = self.insert_entity(
            room_entity_id,
            EntityData {
                act_info: ActInfo {
                    keyword,
                    short_description,
                    gender: Gender::Male,
                },
                internal_title,
                external_description,
                internal_description,
                lateral_description,
                entity_type: EntityType::Player,
                equipped: None,
            },
        );

        self.player_entities
            .insert(name.to_string(), player_entity_id.id);
        self.player_locations
            .insert(name.to_string(), room_entity_id.id);
    }

    fn check_entity_era(&self, entity_id: EntityId) {
        if entity_id.era != self.era {
            panic!("Entity IDs should not be stored long-term!");
        }
    }

    fn entity(&self, entity_id: EntityId) -> &Entity {
        self.check_entity_era(entity_id);
        self.entities
            .get(&entity_id.id)
            .expect("Entities should not be deleted within an era")
    }

    fn entity_mut(&mut self, entity_id: EntityId) -> &mut Entity {
        self.check_entity_era(entity_id);
        self.entities
            .get_mut(&entity_id.id)
            .expect("Entities should not be deleted within an era")
    }

    fn entity_info_raw(&self, entity_id: RawEntityId) -> EntityInfo<'_> {
        let entity = self
            .entities
            .get(&entity_id)
            .expect("Internally constructed IDs should be correct");

        EntityInfo {
            data: &entity.data,
            entity: &entity,
            interner: &self.interner,
            entity_world: &self,
        }
    }

    pub fn entity_info(&self, entity_id: EntityId) -> EntityInfo<'_> {
        let entity = self.entity(entity_id);

        EntityInfo {
            data: &entity.data,
            entity: &entity,
            interner: &self.interner,
            entity_world: &self,
        }
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
            entity_type: room.data.entity_type,
        }
    }

    pub fn player_entity_id(&self, name: &str) -> Option<EntityId> {
        self.player_entities
            .get(name)
            .map(|raw_entity_id| EntityId {
                id: *raw_entity_id,
                era: self.era,
                entity_type: EntityType::Player,
            })
    }

    pub fn world_entity_id(&self) -> EntityId {
        let world_entity = &self.entities[&self.world_entity_id];
        EntityId {
            id: self.world_entity_id,
            era: self.era,
            entity_type: world_entity.data.entity_type,
        }
    }

    fn intern(&mut self, string: &str) -> IntStr {
        IntStr {
            symbol: self.interner.get_or_intern(string),
        }
    }

    fn insert_entity(&mut self, container: EntityId, data: EntityData) -> EntityId {
        self.check_entity_era(container);

        let entity_type = data.entity_type;
        let raw_entity_id = self.id_generator.next();

        let new_entity = Entity {
            data,
            raw_entity_id,
            contents: Vec::new(),
            contained_by: Some(container.id),
        };

        self.entities.insert(raw_entity_id, new_entity);

        let container_entity = self.entity_mut(container);
        container_entity.contents.push(raw_entity_id);

        EntityId {
            id: raw_entity_id,
            era: self.era,
            entity_type,
        }
    }

    pub(crate) fn from_world(world: &World) -> EntityWorld {
        let mut entity_world = EntityWorld::new();
        let mut room_vnum_to_id = HashMap::new();

        for room in &world.rooms {
            let mut intern = |string: &str| IntStr {
                symbol: entity_world.interner.get_or_intern(string),
            };

            let mut room_contents =
                Vec::with_capacity(room.exits.len() + room.extra_descriptions.len());

            for exit in &room.exits {
                room_contents.push(EntityData {
                    act_info: ActInfo {
                        keyword: intern(&exit.name),
                        // TODO: Cow
                        short_description: intern(&format!("the {} exit", exit.name)),
                        gender: Gender::Neutral,
                    },
                    internal_title: intern(&format!("Inside an {} exit.", exit.name)),
                    external_description: intern(
                        exit.description
                            .as_deref()
                            .unwrap_or("You don't see anything special in that direction."),
                    ),
                    internal_description: intern(&format!(
                        "You are inside an {} exit. That normally shouldn't be possible.",
                        exit.name
                    )),
                    lateral_description: intern(&format!("An exit leading {} is here.", exit.name)),
                    entity_type: EntityType::Exit,
                    equipped: None,
                })
            }

            for extra_description in &room.extra_descriptions {
                room_contents.push(EntityData {
                    act_info: ActInfo {
                        keyword: intern(&extra_description.keyword),
                        // TODO: SmallVec
                        short_description: intern(&format!(
                            "extra description called '{}'",
                            extra_description.keyword
                        )),
                        gender: Gender::Neutral,
                    },
                    internal_title: intern(&format!("Inside an extra description.")),
                    external_description: intern(&extra_description.description),
                    internal_description: intern(&format!(
                        "You are inside an extra description. That normally shouldn't be possible."
                    )),
                    lateral_description: intern(&format!(
                        "An extra description called '{}' is here.",
                        extra_description.keyword
                    )),
                    entity_type: EntityType::ExtraDescription,
                    equipped: None,
                })
            }

            let room_data = EntityData {
                act_info: ActInfo {
                    keyword: intern(&room.name),
                    short_description: intern(&room.name),
                    gender: Gender::Neutral,
                },
                internal_title: intern(&room.name),
                external_description: intern(&format!("It's a room called '{}'.", room.name)),
                internal_description: intern(&room.description),
                lateral_description: intern(&format!("A room called '{}' is here.", room.name)),
                entity_type: EntityType::Room,
                equipped: None,
            };

            let room_id = entity_world.insert_entity(entity_world.world_entity_id(), room_data);

            for entity_data in room_contents {
                entity_world.insert_entity(room_id, entity_data);
            }

            room_vnum_to_id.insert(room.vnum.0, room_id);
        }

        entity_world.starting_room = room_vnum_to_id
            .get(&23611)
            .expect("Starting room not found.")
            .id;

        for (_area_data, area_resets) in &world.areas {
            let mut last_mobile_id = None;

            for reset_command in area_resets {
                match reset_command {
                    ResetCommand::Mob {
                        m_num,
                        global_limit: _,
                        r_num,
                        room_limit: _,
                    } => {
                        let room_entity_id = room_vnum_to_id[&r_num.0];
                        let mobile = world.mobile(*m_num);

                        let objective_pronoun = match mobile.gender {
                            Gender::Male => "him",
                            Gender::Female => "her",
                            Gender::Neutral => "it",
                        };

                        let keyword = entity_world.intern(&mobile.name);
                        let internal_title =
                            entity_world.intern(&format!("Inside {}.", mobile.short_description));
                        let short_description = entity_world.intern(&mobile.short_description);
                        let external_description = entity_world.intern(&mobile.description);
                        let internal_description = entity_world.intern(&format!(
                            "You are inside {}. How did you get into {}?",
                            mobile.short_description, objective_pronoun
                        ));
                        let lateral_description = entity_world.intern(&mobile.long_description);

                        let mobile_entity_id = entity_world.insert_entity(
                            room_entity_id,
                            EntityData {
                                act_info: ActInfo {
                                    keyword,
                                    short_description,
                                    gender: mobile.gender.clone(),
                                },
                                internal_title,
                                external_description,
                                internal_description,
                                lateral_description,
                                entity_type: EntityType::Mobile,
                                equipped: None,
                            },
                        );
                        last_mobile_id = Some(mobile_entity_id);
                    }
                    ResetCommand::Object {
                        o_num,
                        global_limit: _,
                        r_num,
                    } => {
                        let room_entity_id = room_vnum_to_id[&r_num.0];
                        let object = world.object(*o_num);

                        load_object(object, room_entity_id, &mut entity_world);
                    }
                    ResetCommand::Door { .. } => {}
                    ResetCommand::Give {
                        o_num,
                        global_limit: _,
                    } => {
                        let last_mobile_id = last_mobile_id.unwrap();
                        let object = world.object(*o_num);

                        load_object(object, last_mobile_id, &mut entity_world);
                    }
                    ResetCommand::Equip {
                        o_num,
                        global_limit: _,
                        location,
                    } => {
                        let last_mobile_id = last_mobile_id.unwrap();
                        let object = world.object(*o_num);

                        let object_id = load_object(object, last_mobile_id, &mut entity_world);
                        let location = entity_world.intern(location);
                        let object_entity = entity_world.entity_mut(object_id);
                        object_entity.data.equipped = Some(location);
                    }
                }
            }
        }
        entity_world
    }
}

fn load_object(object: &Object, container: EntityId, entity_world: &mut EntityWorld) -> EntityId {
    let keyword = entity_world.intern(&object.name);
    let short_description = entity_world.intern(&object.short_description);
    let internal_title = entity_world.intern(&format!("Inside {}.", object.short_description));
    let external_description = entity_world.intern(&object.description);
    let internal_description = entity_world.intern(&format!(
        "You are inside {}. How did you get into it?",
        object.short_description
    ));
    let lateral_description = entity_world.intern(&object.description); // Not ideal.


    let object_id = entity_world.insert_entity(container, EntityData {
        act_info: ActInfo {
            keyword,
            short_description,
            gender: Gender::Neutral,
        },
        internal_title,
        external_description,
        internal_description,
        lateral_description,
        entity_type: EntityType::Object,
        equipped: None,
    });

    for extra_description in &object.extra_descriptions {
        let keyword = entity_world.intern(&extra_description.keyword);
        let short_description = entity_world.intern(&format!(
            "extra description called '{}'",
            extra_description.keyword
        ));

        let internal_title = entity_world.intern(&format!("Inside an object extra description."));
        let external_description = entity_world.intern(&extra_description.description);
        let internal_description = entity_world.intern(&format!(
            "You are inside an object's extra description. That normally shouldn't be possible."
        ));
        let lateral_description = entity_world.intern(&format!(
            "An extra description called '{}' is here.",
            extra_description.keyword
        ));

        entity_world.insert_entity(object_id, EntityData {
            act_info: ActInfo {
                keyword,
                short_description,
                gender: Gender::Neutral,
            },
            internal_title,
            external_description,
            internal_description,
            lateral_description,
            entity_type: EntityType::ExtraDescription,
            equipped: None,
        });
    }

    object_id
}

#[repr(transparent)]
struct IntStr {
    symbol: string_interner::symbol::SymbolU32,
}

struct Entity {
    data: EntityData,
    raw_entity_id: RawEntityId,
    contents: Vec<RawEntityId>,
    contained_by: Option<RawEntityId>,
}

pub(crate) struct EntityData {
    act_info: ActInfo,
    internal_title: IntStr,
    external_description: IntStr,
    internal_description: IntStr,
    lateral_description: IntStr,

    entity_type: EntityType,
    equipped: Option<IntStr>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntityId {
    id: RawEntityId,
    era: u16,
    entity_type: EntityType,
}

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
struct RawEntityId {
    id: NonZeroUsize,
}

pub(crate) struct ActInfo {
    keyword: IntStr,
    short_description: IntStr,
    gender: Gender,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntityType {
    Player,
    Mobile,
    Object,
    Room,
    Exit,
    ExtraDescription,
}

pub(crate) struct EntityInfo<'e> {
    pub data: &'e EntityData,
    entity: &'e Entity,
    interner: &'e StringInterner,
    entity_world: &'e EntityWorld,
}

pub(crate) enum Found<'a> {
    Myself,
    Other(EntityInfo<'a>),
    WrongSelf,
    WrongOther(EntityInfo<'a>),
    Nothing,
}

impl<'e> EntityInfo<'e> {
    pub fn entity_id(&self) -> EntityId {
        EntityId {
            id: self.entity.raw_entity_id,
            era: self.entity_world.era,
            entity_type: self.data.entity_type,
        }
    }

    fn resolve(&self, intstr: &IntStr) -> &'e str {
        self.interner
            .resolve(intstr.symbol)
            .expect("String should be interned on entity creation")
    }

    pub fn short_description(&self) -> &'e str {
        self.resolve(&self.data.act_info.short_description)
    }

    pub fn internal_title(&self) -> &'e str {
        self.resolve(&self.data.internal_title)
    }

    pub fn external_description(&self) -> &'e str {
        self.resolve(&self.data.external_description)
    }

    pub fn internal_description(&self) -> &'e str {
        self.resolve(&self.data.internal_description)
    }

    pub fn lateral_description(&self) -> &'e str {
        self.resolve(&self.data.lateral_description)
    }

    pub fn gender(&self) -> Gender {
        self.data.act_info.gender
    }

    pub fn keyword(&self) -> &'e str {
        self.resolve(&self.data.act_info.keyword)
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
        let interner = self.interner;

        self.entity.contents.iter().filter_map(move |entity_id| {
            let entity = &entity_world.entities[entity_id];
            if entity.data.entity_type == entity_type {
                Some(EntityInfo {
                    data: &entity.data,
                    entity,
                    interner,
                    entity_world,
                })
            } else {
                None
            }
        })
    }

    pub fn contained_entities(&self) -> impl Iterator<Item = EntityInfo<'e>> {
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

        for entity in room.contained_entities() {
            if entity
                .keyword()
                .split_whitespace()
                .any(|word| word == keyword)
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
