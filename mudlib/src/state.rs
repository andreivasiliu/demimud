use std::collections::BTreeMap;

use inflector::Inflector;

use crate::{
    players::Players,
    socials::Socials,
    world::{ResetCommand, Vnum, World},
};

#[derive(Default, Clone)]
struct RoomState {
    vnum: Vnum,

    players: BTreeMap<String, PlayerState>,
    objects: Vec<ObjectState>,
    mobiles: Vec<MobileState>,
}

#[derive(Default, Clone)]
struct PlayerState {}

#[derive(Default, Clone)]
struct ObjectState {
    vnum: Vnum,
}

#[derive(Default, Clone)]
struct MobileState {
    vnum: Vnum,
}

pub(super) struct WorldState {
    world: World,
    pub(crate) socials: Socials,

    pub(crate) players: Players,
    rooms: Vec<RoomState>,
}

pub(super) fn create_state(world: World, socials: Socials) -> WorldState {
    let players = Players {
        locations: Default::default(),
        echoes: Default::default(),
        current_player: Default::default(),
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
            for reset_command in area_resets {
                match reset_command {
                    ResetCommand::Mob {
                        m_num,
                        global_limit: _,
                        r_num,
                        room_limit: _,
                    } => {
                        let room = &mut self.rooms[r_num.0];
                        room.mobiles.push(MobileState { vnum: *m_num });
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
            self.rooms[3000]
                .players
                .insert(name.clone(), PlayerState {});
            self.players.locations.insert(name.clone(), Vnum(3000));
        }
        if !self.players.echoes.contains_key(&name) {
            self.players.echoes.insert(name, String::new());
        }
    }

    pub fn do_look_at(&mut self, target: &str) {
        let location = self.players.locations[&self.players.current_player];

        let description = match self.find_description(location, target) {
            None => "You don't see anything named like that here.\r\n".to_string(),
            Some(description) => description,
        };

        self.players.current().echo(description);
    }

    pub fn find_description(&self, location: Vnum, target: &str) -> Option<String> {
        let room_state = &self.rooms[location.0];
        let room = self.world.room(location);

        for exit in &room.exits {
            if exit.name == target {
                if let Some(description) = &exit.description {
                    return Some(description.clone());
                } else {
                    return Some(format!(
                        "You don't see anything special to the {}.\r\n",
                        target
                    ));
                }
            }
        }

        for object_state in &room_state.objects {
            let object = self.world.object(object_state.vnum);

            for extra_description in &object.extra_descriptions {
                if extra_description
                    .keyword
                    .split_whitespace()
                    .find(|&word| word == target)
                    .is_some()
                {
                    return Some(extra_description.description.clone());
                }
            }

            if object
                .name
                .split_whitespace()
                .find(|&word| word == target)
                .is_some()
            {
                return Some(object.description.clone() + "\r\n");
            }
        }

        for mobile_state in &room_state.mobiles {
            let mobile = self.world.mobile(mobile_state.vnum);

            if mobile
                .name
                .split_whitespace()
                .find(|&word| word == target)
                .is_some()
            {
                return Some(mobile.description.clone());
            }
        }

        for extra_description in &room.extra_descriptions {
            if extra_description
                .keyword
                .split_whitespace()
                .find(|&word| word == target)
                .is_some()
            {
                return Some(extra_description.description.clone());
            }
        }

        for player in room_state.players.keys() {
            if player == target {
                return Some(format!(
                    "{} is a player. Players don't yet have a description.\r\n",
                    player.to_pascal_case()
                ));
            }
        }

        None
    }

    pub fn do_look(&mut self) {
        let location = self.players.locations[&self.players.current_player];
        let room_state = &self.rooms[location.0];
        let room = self.world.room(location);

        use std::fmt::Write;
        let mut output = self.players.current();

        write!(output, "\x1b[33m{}\x1b[0m\r\n", room.name).unwrap();
        write!(output, "{}", room.description).unwrap();

        if room.exits.is_empty() {
            write!(output, "\x1b[32mYou see no exits.\x1b[0m\r\n").unwrap();
        } else {
            write!(output, "\x1b[32mYou see exits: ").unwrap();
            let mut first = true;

            for exit in room.exits.iter() {
                if first {
                    first = false;
                } else {
                    write!(output, ", ").unwrap();
                }
                write!(output, "{}", exit.name).unwrap();
            }
            write!(output, ".\x1b[0m\r\n").unwrap();
        }

        for object in &room_state.objects {
            write!(
                output,
                "\x1b[36m{}\x1b[0m\r\n",
                self.world.object(object.vnum).description
            )
            .unwrap();
        }

        for mob in &room_state.mobiles {
            if !self.world.mobile(mob.vnum).unseen {
                write!(
                    output,
                    "\x1b[35m{}\x1b[0m\r\n",
                    self.world.mobile(mob.vnum).long_description
                )
                .unwrap();
            }
        }

        for player in room_state.players.keys() {
            if player == &self.players.current_player {
                continue;
            }

            let proper_name = player.to_pascal_case();
            write!(
                self.players.current(),
                "\x1b[1;35m{}, a player, is here.\x1b[0m\r\n",
                proper_name
            )
            .unwrap();
        }
    }

    pub fn do_move(&mut self, direction: &str) -> bool {
        let location = self.players.locations[&self.players.current_player];
        let old_room = self.world.room(location);

        let direction = long_direction(direction);

        use std::fmt::Write;

        if let Some(exit) = old_room.exits.iter().find(|e| e.name == direction) {
            let new_location = exit.vnum;

            if self.world.has_room(new_location) {
                let old_room_state = &mut self.rooms[location.0];
                let (name, player) = old_room_state
                    .players
                    .remove_entry(&self.players.current_player)
                    .unwrap();
                let new_room_state = &mut self.rooms[new_location.0];
                new_room_state.players.insert(name, player);

                let player = self.players.current_player.to_title_case();

                write!(self.players.current(), "You walk {}.\r\n", direction).unwrap();
                write!(
                    self.players.others(),
                    "{} leaves {}.\r\n",
                    player,
                    direction
                )
                .unwrap();

                self.players.current().change_player_location(new_location);

                write!(
                    self.players.others(),
                    "{} arrives from the {}.\r\n",
                    player,
                    opposite_direction(direction)
                )
                .unwrap();

                self.do_look();
            } else {
                write!(
                    self.players.current(),
                    "The way {} leads into the void!\r\n",
                    direction
                )
                .unwrap();
            }
        } else {
            if common_direction(direction) {
                write!(
                    self.players.current(),
                    "The way to the {} is blocked.\r\n",
                    direction
                )
                .unwrap();
            } else {
                return false;
            }
        }
        true
    }

    pub(crate) fn do_say(&mut self, message: &str) {
        use std::fmt::Write;

        let first_character = message.chars().next();

        if let Some(character) = first_character {
            let bytes = character.len_utf8();
            let remaining_characters = &message[bytes..];

            let ends_in_punctuation = message
                .chars()
                .last()
                .map(|c: char| !c.is_alphanumeric())
                .unwrap_or(true);

            let uppercase_character = character.to_uppercase();

            let suffix = if ends_in_punctuation { "" } else { "." };

            let mut output = self.players.current();
            write!(
                output,
                "\x1b[1;35mYou say, '{}{}{}'\x1b[0m\r\n",
                uppercase_character, remaining_characters, suffix
            )
            .unwrap();

            let name = self.players.current_player.to_title_case();

            let mut output = self.players.others();
            write!(
                output,
                "\x1b[1;35m{} says, '{}{}{}'\x1b[0m\r\n",
                name, uppercase_character, remaining_characters, suffix
            )
            .unwrap();
        } else {
            self.players
                .current()
                .echo("You say nothing whatsoever.\r\n");
        }
    }
}

fn opposite_direction(direction: &str) -> &str {
    match direction {
        "north" => "south",
        "east" => "west",
        "south" => "south",
        "west" => "east",
        "up" => "down",
        "down" => "up",
        "northeast" => "southwest",
        "southeast" => "northwest",
        "southwest" => "northeast",
        "northwest" => "southeast",
        name => name,
    }
}

fn long_direction(direction: &str) -> &str {
    match direction {
        "n" => "north",
        "e" => "east",
        "s" => "south",
        "w" => "west",
        "u" => "up",
        "d" => "down",
        "ne" => "northeast",
        "se" => "southeast",
        "sw" => "southwest",
        "nw" => "northwest",
        dir => dir,
    }
}

fn short_direction(direction: &str) -> &str {
    match direction {
        "north" => "n",
        "east" => "e",
        "south" => "s",
        "west" => "w",
        "up" => "u",
        "down" => "d",
        "northeast" => "n",
        "southeast" => "s",
        "southwest" => "s",
        "northwest" => "n",
        dir => dir,
    }
}

fn common_direction(direction: &str) -> bool {
    let common_directions = &["n", "e", "s", "w", "u", "d", "ne", "se", "sw", "nw"];

    common_directions.contains(&short_direction(direction))
}

fn random_bits(bits: u8) -> bool {
    (rand::random::<u32>() >> 7) & ((1u32 << bits) - 1) == 0
}
