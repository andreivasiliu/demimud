use std::collections::BTreeMap;

use inflector::Inflector;

use crate::{
    players::Players,
    socials::Socials,
    world::{Exit, ExtraDescription, Mobile, Object, ResetCommand, Room, Vnum, World, opposite_direction},
};

#[derive(Default, Clone)]
pub(crate) struct RoomState {
    pub(crate) vnum: Vnum,

    pub(crate) players: BTreeMap<String, PlayerState>,
    pub(crate) objects: Vec<ObjectState>,
    pub(crate) mobiles: Vec<MobileState>,
}

#[derive(Default, Clone)]
pub(crate) struct PlayerState {
    pub(crate) character: CharacterState,
}

#[derive(Default, Clone)]
pub(crate) struct ObjectState {
    pub(crate) vnum: Vnum,
}

#[derive(Default, Clone)]
pub(crate) struct MobileState {
    pub(crate) vnum: Vnum,
    pub(crate) character: CharacterState,
}

#[derive(Default, Clone)]
pub(crate) struct CharacterState {
    pub(crate) inventory: Vec<ObjectState>,
    pub(crate) equipment: Vec<(String, ObjectState)>,
}

pub(super) struct WorldState {
    pub(crate) world: World,
    pub(crate) socials: Socials,

    pub(crate) players: Players,
    pub(crate) rooms: Vec<RoomState>,
}

pub(super) fn create_state(world: World, socials: Socials) -> WorldState {
    let players = Players {
        locations: Default::default(),
        echoes: Default::default(),
        current_player: Default::default(),
        current_target: None,
    };

    let mut rooms = Vec::new();
    rooms.resize(world.rooms.len(), RoomState::default());

    for (index, room) in rooms.iter_mut().enumerate() {
        room.vnum = world.rooms[index].vnum;
    }

    WorldState {
        world,
        socials,
        players,
        rooms,
    }
}

impl WorldState {
    pub(super) fn reset_world(&mut self) {
        for room in &mut self.rooms {
            room.objects.clear();
            room.mobiles.clear();
        }

        for (_area_data, area_resets) in &self.world.areas {
            let mut last_mobile = None;

            for reset_command in area_resets {
                match reset_command {
                    ResetCommand::Mob {
                        m_num,
                        global_limit: _,
                        r_num,
                        room_limit: _,
                    } => {
                        let room = &mut self.rooms[r_num.0];
                        room.mobiles.push(MobileState {
                            vnum: *m_num,
                            character: Default::default(),
                        });
                        last_mobile = Some((r_num.0, room.mobiles.len() - 1));
                    }
                    ResetCommand::Object {
                        o_num,
                        global_limit: _,
                        r_num,
                    } => {
                        let room = &mut self.rooms[r_num.0];
                        room.objects.push(ObjectState { vnum: *o_num });
                    }
                    ResetCommand::Door { .. } => {}
                    ResetCommand::Give {
                        o_num,
                        global_limit: _,
                    } => {
                        let last_mobile = last_mobile.unwrap();
                        let room = &mut self.rooms[last_mobile.0];
                        let mob = &mut room.mobiles[last_mobile.1];

                        mob.character.inventory.push(ObjectState { vnum: *o_num })
                    }
                    ResetCommand::Equip {
                        o_num,
                        global_limit: _,
                        location,
                    } => {
                        let last_mobile = last_mobile.unwrap();
                        let room = &mut self.rooms[last_mobile.0];
                        let mob = &mut room.mobiles[last_mobile.1];

                        mob.character
                            .equipment
                            .push((location.clone(), ObjectState { vnum: *o_num }));
                    }
                }
            }
        }
    }

    pub(super) fn update_world(&mut self) {
        let mut limbo = Vec::new();
        let world = &self.world;
        let players = &mut self.players;

        for room_state in &mut self.rooms {
            let current_vnum = room_state.vnum;
            room_state.mobiles.retain(|mobile_state| {
                let mobile = world.mobile(mobile_state.vnum);
                let room = world.room(current_vnum);

                if !mobile.sentinel && random_bits(4) && !room.exits.is_empty() {
                    let random_exit = rand::random::<usize>() % room.exits.len();
                    let random_chance = rand::random::<usize>() % 10;

                    // The original implementation skipped on non-existent exits.
                    if room.exits.len() < random_chance {
                        return true;
                    }

                    players.npc(room.vnum).act(format!(
                        "{} wanders to the {}.\r\n",
                        mobile.short_description.to_sentence_case(),
                        room.exits[random_exit].name,
                    ));

                    limbo.push((
                        mobile_state.clone(),
                        room.vnum,
                        room.exits[random_exit].vnum,
                        &room.exits[random_exit].name,
                    ));
                    false
                } else {
                    true
                }
            });
        }

        for (mobile_state, current_vnum, target_vnum, exit_name) in limbo {
            let returned;
            let target_room;

            if self.world.has_room(target_vnum) {
                returned = false;
                target_room = &mut self.rooms[target_vnum.0];
            } else {
                returned = true;
                target_room = &mut self.rooms[current_vnum.0];
            };

            let mobile_vnum = mobile_state.vnum;

            target_room.mobiles.push(mobile_state);

            let mobile = self.world.mobile(mobile_vnum);
            let mut output = self.players.npc(target_room.vnum);

            if returned {
                output.act(format!(
                    "With a confused look, {} wanders back in.\r\n",
                    mobile.short_description
                ));
            } else {
                output.act(format!(
                    "{} arrives from the {}.\r\n",
                    mobile.short_description.to_sentence_case(),
                    opposite_direction(exit_name)
                ));
            }
        }
    }

    pub(super) fn add_player(&mut self, name: String) {
        if !self.players.locations.contains_key(&name) {
            self.rooms[23611].players.insert(
                name.clone(),
                PlayerState {
                    character: Default::default(),
                },
            );
            self.players.locations.insert(name.clone(), Vnum(23611));
        }
        self.players.echoes.entry(name).or_insert(String::new());
    }
}

pub(crate) fn find_description<'a>(
    players: &'a Players,
    rooms: &'a [RoomState],
    world: &'a World,
    target: &'a str,
) -> Option<String> {
    let target = find_target(players, rooms, world, target);

    match target {
        Target::Me => {
            Some("It's you! But you're sadly just a player and players don't have descriptions yet.\r\n".to_string())
        }
        Target::Exit(exit) => {
            if let Some(description) = &exit.description {
                Some(description.clone())
            } else {
                Some(format!(
                    "You don't see anything special to the {}.\r\n",
                    &exit.name
                ))
            }
        }
        Target::Object(object) => {
            Some(object.description.clone() + "\r\n")
        }
        Target::Mobile(mobile, mobile_state) => {
            let mut description = String::new();
            description += &mobile.description;
            let mut first = true;
            for item in &mobile_state.character.inventory {
                if first {
                    first = false;
                    description += &format!("{} is holding:\r\n", &mobile.short_description.to_sentence_case());
                }
                let object = world.object(item.vnum);
                description += &format!("`c{}`^\r\n", object.short_description);
            }
            let mut first = true;
            for (location, item) in &mobile_state.character.equipment {
                if first {
                    first = false;
                    description += &format!("{} is wearing:\r\n", &mobile.short_description.to_sentence_case());
                }
                let object = world.object(item.vnum);
                description += &format!(" `S[`y{}`S]:`^ {}`^\r\n", location, object.short_description);
            }
            Some(description)
        }
        Target::Player(player) => {
            Some(format!(
                "{} is a player. Players don't yet have a description.\r\n",
                player.to_pascal_case()
            ))
        }
        Target::ObjectExtraDescription(_object, extra_description) => {
            Some(extra_description.description.clone())
        }
        Target::RoomExtraDescription(_room, extra_description) => {
            Some(extra_description.description.clone())
        }
        Target::NotFound(_target) => {
            None
        }
    }
}

pub(crate) enum Target<'a> {
    Me,
    Exit(&'a Exit),
    Object(&'a Object),
    Mobile(&'a Mobile, &'a MobileState),
    Player(&'a str),
    ObjectExtraDescription(&'a Object, &'a ExtraDescription),
    RoomExtraDescription(&'a Room, &'a ExtraDescription),
    NotFound(&'a str),
}

pub(crate) fn find_target<'a>(
    players: &'a Players,
    rooms: &'a [RoomState],
    world: &'a World,
    target: &'a str,
) -> Target<'a> {
    let location = players.locations[&players.current_player];
    let room_state = &rooms[location.0];
    let room = world.room(location);

    if target.eq_ignore_ascii_case(&players.current_player) {
        return Target::Me;
    }

    if target == "self" || target == "myself" || target == "me" {
        return Target::Me;
    }

    for exit in &room.exits {
        if exit.name == target {
            return Target::Exit(&exit);
        }
    }

    for object_state in &room_state.objects {
        let object = world.object(object_state.vnum);

        for extra_description in &object.extra_descriptions {
            if extra_description
                .keyword
                .split_whitespace()
                .any(|word| word == target)
            {
                return Target::ObjectExtraDescription(&object, &extra_description);
            }
        }

        if object
            .name
            .split_whitespace()
            .any(|word| word == target)
        {
            return Target::Object(&object);
        }
    }

    for mobile_state in &room_state.mobiles {
        let mobile = world.mobile(mobile_state.vnum);

        if mobile
            .name
            .split_whitespace()
            .any(|word| word == target)
        {
            return Target::Mobile(&mobile, &mobile_state);
        }
    }

    for extra_description in &room.extra_descriptions {
        if extra_description
            .keyword
            .split_whitespace()
            .any(|word| word == target)
        {
            return Target::RoomExtraDescription(&room, &extra_description);
        }
    }

    for player in room_state.players.keys() {
        if player == target {
            return Target::Player(player);
        }
    }

    Target::NotFound(target)
}

pub(crate) fn change_player_location(
    rooms: &mut [RoomState],
    players: &mut Players,
    new_location: Vnum,
) -> () {
    let location = players.locations[&players.current_player];
    let old_room_state = &mut rooms[location.0];
    let (name, player) = old_room_state
        .players
        .remove_entry(&players.current_player)
        .unwrap();
    let new_room_state = &mut rooms[new_location.0];
    new_room_state.players.insert(name, player);

    players.current().change_player_location(new_location);
}

fn random_bits(bits: u8) -> bool {
    (rand::random::<u32>() >> 7) & ((1u32 << bits) - 1) == 0
}
