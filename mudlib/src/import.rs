//! Convert a DoW world to EntityWorld entities

use std::collections::HashMap;

use string_interner::StringInterner;

use crate::{components::{Components, Door, EntityType, GeneralData, InternComponent, MobProg}, entity::{EntityId, EntityWorld, PermanentEntityId}, world::{Gender, MobProgTrigger, Mobile, Object, ResetCommand, World}};

pub(crate) struct VnumTemplates {
    pub vnum_to_entity: Vec<Option<PermanentEntityId>>,
    pub vnum_to_mobprog: Vec<Option<String>>,
    pub object_components: Vec<Option<(Components, Vec<Components>)>>,
    pub mobile_components: Vec<Option<(Components, Vec<Components>)>>,
}

pub(crate) fn import_from_world(entity_world: &mut EntityWorld, world: &World) -> VnumTemplates {
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
                    command_queue: Vec::new(),
                },
                mobile: None,
                object: None,
                door: None,
                mobprog: None,
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

            let door = if exit.has_door {
                Some(Door {
                    closed: exit.is_closed,
                    locked: exit.is_locked,
                    key: exit.key,
                })
            } else {
                None
            };

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
                    command_queue: Vec::new(),
                },
                mobile: None,
                object: None,
                door,
                mobprog: None,
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
                    command_queue: Vec::new(),
                },
                mobile: None,
                object: None,
                door: None,
                mobprog: None,
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

    let landmarks = &[
        ("gnomehill", 23611),
        ("mekali", 3000),
        ("dzagari", 27003),
        ("mudschool", 7371),
    ];

    for (landmark, vnum) in landmarks {
        entity_world.add_landmark(
            landmark,
            *room_vnum_to_id
                .get(&vnum)
                .expect("GnomeHill landmark room not found."),
        );
    }

    let mut vnum_templates = VnumTemplates {
        vnum_to_entity: Vec::with_capacity(world.rooms.len()),
        vnum_to_mobprog: Vec::with_capacity(world.mobprogs.len()),
        object_components: Vec::with_capacity(world.objects.len()),
        mobile_components: Vec::with_capacity(world.mobiles.len()),
    };

    vnum_templates.vnum_to_entity.resize(world.rooms.len(), None);
    vnum_templates.vnum_to_mobprog.resize(world.rooms.len(), None);
    vnum_templates.object_components.resize(world.rooms.len(), None);
    vnum_templates.mobile_components.resize(world.rooms.len(), None);

    for room in &world.rooms {
        if room.vnum.0 != 0 {
            let room_entity = entity_world.entity_info(room_vnum_to_id[&room.vnum.0]);
            vnum_templates.vnum_to_entity[room.vnum.0] = Some(room_entity.permanent_entity_id());
        }
    }

    for object in &world.objects {
        if object.vnum.0 != 0 {
            let components = import_object_components(object, &mut entity_world.interner);
            vnum_templates.object_components[object.vnum.0] = Some(components);
        }
    }

    for mobile in &world.mobiles {
        if mobile.vnum.0 != 0 {
            let components = import_mobile_components(mobile, world, &mut entity_world.interner);
            vnum_templates.mobile_components[mobile.vnum.0] = Some(components);
        }
    }

    for mobprog in &world.mobprogs {
        vnum_templates.vnum_to_mobprog[mobprog.vnum.0] = Some(mobprog.code.clone());
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
                    let mobile_components = vnum_templates.mobile_components[m_num.0].as_ref().expect("Mobile with vnum does not exist");

                    let mobile_entity_id = entity_world.insert_entity(
                        room_entity_id,
                        mobile_components.0.clone(),
                    );
                    last_mobile_id = Some(mobile_entity_id);

                    for mobprog_components in &mobile_components.1 {
                        entity_world.insert_entity(
                            mobile_entity_id,
                            mobprog_components.clone(),
                        );
                    }
                }
                ResetCommand::Object {
                    o_num,
                    global_limit: _,
                    r_num,
                } => {
                    let room_entity_id = room_vnum_to_id[&r_num.0];
                    load_object(o_num.0, room_entity_id, &vnum_templates, entity_world);
                }
                ResetCommand::Door { .. } => {}
                ResetCommand::Give {
                    o_num,
                    global_limit: _,
                } => {
                    let last_mobile_id = last_mobile_id.unwrap();
                    load_object(o_num.0, last_mobile_id, &vnum_templates, entity_world);
                }
                ResetCommand::Equip {
                    o_num,
                    global_limit: _,
                    location,
                } => {
                    let last_mobile_id = last_mobile_id.unwrap();

                    let object_id = load_object(o_num.0, last_mobile_id, &vnum_templates, entity_world);
                    let location = location.to_string();
                    let mut object_entity = entity_world.entity_info_mut(object_id);
                    object_entity.components().general.equipped = Some(location);
                }
            }
        }
    }

    vnum_templates
}

fn import_mobile_components(mobile: &Mobile, world: &World, interner: &mut StringInterner) -> (Components, Vec<Components>) {
    let mut mobprogs = Vec::with_capacity(mobile.mobprog_triggers.len());

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

    let act_info = interner
        .act_info(keyword, &short_description, mobile.gender);
    let descriptions = interner
        .descriptions(&title, &internal, external, lateral);

    let shop = world.shops.get(mobile.vnum.0).filter(|shop| shop.vnum.0 != 0);

    let mobile_components = Components {
        act_info,
        descriptions,
        general: GeneralData {
            area: mobile.area.to_string(),
            sector: None,
            entity_type: EntityType::Mobile,
            equipped: None,
            command_queue: Vec::new(),
        },
        mobile: Some(crate::components::Mobile {
            wander: !mobile.sentinel,
            shopkeeper: shop.cloned(),
        }),
        object: None,
        door: None,
        mobprog: None,
    };

    for (mobprog_trigger, vnum) in &mobile.mobprog_triggers {
        let mobprog = world
            .mobprogs
            .get(vnum.0)
            .expect("Triggered mobprog should exist");
        let keyword = "mobprog";
        let trigger = match mobprog_trigger {
            MobProgTrigger::Random { .. } => "on-chance",
            MobProgTrigger::Speech { .. } => "on-speech",
            MobProgTrigger::Greet { .. } => "on-greet",
            MobProgTrigger::Entry { .. } => "on-entry",
            MobProgTrigger::Act { .. } => "on-act",
            MobProgTrigger::Exit { .. } => "on-exit",
            MobProgTrigger::Bribe { .. } => "on-bribe",
            MobProgTrigger::Give { .. } => "on-give",
            MobProgTrigger::Kill { .. } => "on-kill",
            MobProgTrigger::Death { .. } => "on-death",
            MobProgTrigger::Hour { .. } => "on-hour",
            MobProgTrigger::LoginRoom { .. } => "on-login",
        };
        let short_description =
            format!("an {} mobprog titled '`S{}`^'", trigger, mobprog.title);

        let title = "Inside a mobprog.";
        let internal = "You are inside a mobprog. Instructions are floating all around the area.";
        let external = format!(
            "It's a mobprog. When triggered, it runs the following code:\r\n{}\r\n",
            mobprog.code
        );
        let lateral = "A mobprog is installed here, affecting its surroundings.";

        let act_info = interner.act_info(
            keyword,
            &short_description,
            Gender::Neutral,
        );
        let descriptions = interner
            .descriptions(title, internal, &external, lateral);

        mobprogs.push(
            Components {
                act_info,
                descriptions,
                general: GeneralData {
                    area: mobile.area.to_string(),
                    sector: None,
                    entity_type: EntityType::MobProg,
                    equipped: None,
                    command_queue: Vec::new(),
                },
                mobile: None,
                object: None,
                door: None,
                mobprog: Some(MobProg {
                    trigger: mobprog_trigger.clone(),
                    code: mobprog.code.clone(),
                }),
            }
        );
    }

    (mobile_components, mobprogs)
}

fn import_object_components(object: &Object, interner: &mut StringInterner) -> (Components, Vec<Components>) {
    let mut extra_description_components = Vec::with_capacity(object.extra_descriptions.len());

    let keyword = &object.name;
    let short_description = &object.short_description;

    let title = format!("Inside {}.", object.short_description);
    let external = &object.description;
    let internal = &format!(
        "You are inside {}. How did you get into it?",
        object.short_description
    );
    let lateral = &object.description; // Not ideal.

    let act_info = interner
        .act_info(keyword, short_description, Gender::Neutral);
    let descriptions = interner
        .descriptions(&title, &internal, external, lateral);

    let components = Components {
        act_info,
        descriptions,
        general: GeneralData {
            area: object.area.to_string(),
            sector: None,
            entity_type: EntityType::Object,
            equipped: None,
            command_queue: Vec::new(),
        },
        mobile: None,
        object: Some(crate::components::Object {
            cost: object.cost,
            key: if object.item_type == "key" {
                Some(object.vnum)
            } else {
                None
            },
        }),
        door: None,
        mobprog: None,
    };

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

        let act_info = interner
            .act_info(keyword, &short_description, Gender::Neutral);
        let descriptions = interner
            .descriptions(&title, &internal, external, &lateral);

        extra_description_components.push(
            Components {
                act_info,
                descriptions,
                general: GeneralData {
                    area: object.area.to_string(),
                    sector: None,
                    entity_type: EntityType::ExtraDescription,
                    equipped: None,
                    command_queue: Vec::new(),
                },
                mobile: None,
                object: None,
                door: None,
                mobprog: None,
            },
        );
    }

    (components, extra_description_components)
}

fn load_object(vnum: usize, container: EntityId, vnum_templates: &VnumTemplates, entity_world: &mut EntityWorld) -> EntityId {
    let components = vnum_templates.object_components[vnum].as_ref().expect("Object with vnum does not exist");

    let object_id = entity_world.insert_entity(
        container,
        components.0.clone(),
    );

    for extra_description_components in &components.1 {
        entity_world.insert_entity(
            object_id,
            extra_description_components.clone(),
        );
    }

    object_id
}
