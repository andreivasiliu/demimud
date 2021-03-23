//! Convert a DoW world to EntityWorld entities

use std::collections::HashMap;

use crate::{
    components::{Components, EntityType, GeneralData, InternComponent},
    entity::{EntityId, EntityWorld},
    world::{Gender, Object, ResetCommand, World},
};

pub(crate) fn import_from_world(entity_world: &mut EntityWorld, world: &World) -> () {
    let mut room_vnum_to_id = HashMap::new();
    let mut exit_leads_to = HashMap::new();

    for room in &world.rooms {
        let room_id = {
            let keyword = &room.name;
            let short_description = &room.name;

            let title = &room.name;
            let external = format!("It's a room called '{}'.", room.name);
            let internal = &room.description;
            let lateral = format!("A room called '{}' is here.", room.name);

            let room_components = Components {
                act_info: entity_world.interner.act_info(
                    keyword,
                    short_description,
                    Gender::Neutral,
                ),
                descriptions: entity_world
                    .interner
                    .descriptions(title, internal, &external, &lateral),
                general: GeneralData {
                    area: room.area.to_string(),
                    sector: Some(room.sector.to_string()),
                    entity_type: EntityType::Room,
                    equipped: None,
                },
                mobile: None,
            };

            entity_world.insert_entity(entity_world.world_entity_id(), room_components)
        };

        for exit in &room.exits {
            let keyword = &exit.name;
            let short_description = format!("the {} exit", exit.name);

            let title = format!("Inside an {} exit.", exit.name);
            let external = exit
                .description
                .as_deref()
                .unwrap_or("You don't see anything special in that direction.");
            let internal = format!(
                "You are inside an {} exit. That normally shouldn't be possible.",
                exit.name
            );
            let lateral = format!("An exit leading {} is here.", exit.name);

            let exit_components = Components {
                act_info: entity_world.interner.act_info(
                    keyword,
                    &short_description,
                    Gender::Neutral,
                ),
                descriptions: entity_world
                    .interner
                    .descriptions(&title, &internal, external, &lateral),
                general: GeneralData {
                    area: room.area.to_string(),
                    sector: None,
                    entity_type: EntityType::Exit,
                    equipped: None,
                },
                mobile: None,
            };
            let exit_id = entity_world.insert_entity(room_id, exit_components);

            exit_leads_to.insert(exit_id, exit.vnum.0);
        }

        for extra_description in &room.extra_descriptions {
            let keyword = &extra_description.keyword;
            let short_description =
                format!("extra description called '{}'", extra_description.keyword);

            let title = "Inside an extra description.";
            let external = &extra_description.description;
            let internal =
                "You are inside an extra description. That normally shouldn't be possible.";
            let lateral = format!(
                "An extra description called '{}' is here.",
                extra_description.keyword
            );
            let extra_description_components = Components {
                act_info: entity_world.interner.act_info(
                    keyword,
                    &short_description,
                    Gender::Neutral,
                ),
                descriptions: entity_world
                    .interner
                    .descriptions(title, internal, external, &lateral),
                general: GeneralData {
                    area: room.area.to_string(),
                    sector: None,
                    entity_type: EntityType::ExtraDescription,
                    equipped: None,
                },
                mobile: None,
            };

            entity_world.insert_entity(room_id, extra_description_components);
        }

        room_vnum_to_id.insert(room.vnum.0, room_id);
    }

    for (exit_id, leads_to) in exit_leads_to {
        if let Some(to_room_id) = room_vnum_to_id.get(&leads_to) {
            entity_world.set_leads_to(exit_id, *to_room_id);
        }
    }

    let landmarks = &[("gnomehill", 23611), ("mekali", 3000), ("dzagari", 27003)];

    for (landmark, vnum) in landmarks {
        entity_world.add_landmark(
            landmark,
            *room_vnum_to_id
                .get(&vnum)
                .expect("GnomeHill landmark room not found."),
        );
    }

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

                    let keyword = &mobile.name;
                    let short_description = &mobile.short_description;

                    let title = format!("Inside {}.", mobile.short_description);
                    let external = &mobile.description;
                    let internal = format!(
                        "You are inside {}. How did you get into {}?",
                        mobile.short_description, objective_pronoun
                    );
                    let lateral = &mobile.long_description;

                    let act_info =
                        entity_world
                            .interner
                            .act_info(keyword, &short_description, mobile.gender);
                    let descriptions = entity_world
                        .interner
                        .descriptions(&title, &internal, external, lateral);

                    let mobile_entity_id = entity_world.insert_entity(
                        room_entity_id,
                        Components {
                            act_info,
                            descriptions,
                            general: GeneralData {
                                area: mobile.area.to_string(),
                                sector: None,
                                entity_type: EntityType::Mobile,
                                equipped: None,
                            },
                            mobile: Some(crate::components::Mobile {
                                wander: !mobile.sentinel,
                            }),
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

                    load_object(object, room_entity_id, entity_world);
                }
                ResetCommand::Door { .. } => {}
                ResetCommand::Give {
                    o_num,
                    global_limit: _,
                } => {
                    let last_mobile_id = last_mobile_id.unwrap();
                    let object = world.object(*o_num);

                    load_object(object, last_mobile_id, entity_world);
                }
                ResetCommand::Equip {
                    o_num,
                    global_limit: _,
                    location,
                } => {
                    let last_mobile_id = last_mobile_id.unwrap();
                    let object = world.object(*o_num);

                    let object_id = load_object(object, last_mobile_id, entity_world);
                    let location = location.to_string();
                    let mut object_entity = entity_world.entity_info_mut(object_id);
                    object_entity.components().general.equipped = Some(location);
                }
            }
        }
    }
}

fn load_object(object: &Object, container: EntityId, entity_world: &mut EntityWorld) -> EntityId {
    let keyword = &object.name;
    let short_description = &object.short_description;

    let title = format!("Inside {}.", object.short_description);
    let external = &object.description;
    let internal = &format!(
        "You are inside {}. How did you get into it?",
        object.short_description
    );
    let lateral = &object.description; // Not ideal.

    let act_info = entity_world
        .interner
        .act_info(keyword, short_description, Gender::Neutral);
    let descriptions = entity_world
        .interner
        .descriptions(&title, &internal, external, lateral);

    let object_id = entity_world.insert_entity(
        container,
        Components {
            act_info,
            descriptions,
            general: GeneralData {
                area: object.area.to_string(),
                sector: None,
                entity_type: EntityType::Object,
                equipped: None,
            },
            mobile: None,
        },
    );

    for extra_description in &object.extra_descriptions {
        let keyword = &extra_description.keyword;
        let short_description = format!("extra description called '{}'", extra_description.keyword);

        let title = format!("Inside an object extra description.");
        let external = &extra_description.description;
        let internal = format!(
            "You are inside an object's extra description. That normally shouldn't be possible."
        );
        let lateral = format!(
            "An extra description called '{}' is here.",
            extra_description.keyword
        );

        let act_info = entity_world
            .interner
            .act_info(keyword, &short_description, Gender::Neutral);
        let descriptions = entity_world
            .interner
            .descriptions(&title, &internal, external, &lateral);

        entity_world.insert_entity(
            object_id,
            Components {
                act_info,
                descriptions,
                general: GeneralData {
                    area: object.area.to_string(),
                    sector: None,
                    entity_type: EntityType::ExtraDescription,
                    equipped: None,
                },
                mobile: None,
            },
        );
    }

    object_id
}
