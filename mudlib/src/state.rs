use std::collections::BTreeMap;

use inflector::Inflector;

use crate::world::{ResetCommand, Vnum, World};

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

    player_location: BTreeMap<String, Vnum>,
    rooms: Vec<RoomState>,
}

pub(super) fn create_state(world: World) -> WorldState {
    let player_location = BTreeMap::new();

    let mut rooms = Vec::new();
    rooms.resize(world.rooms.len(), RoomState::default());

    WorldState {
        world,
        player_location,
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

    pub(super) fn add_player(&mut self, name: String) {
        if !self.player_location.contains_key(&name) {
            self.rooms[3000].players.insert(name.clone(), PlayerState {});
            self.player_location.insert(name, Vnum(3000));
        }
    }

    pub fn do_look_at(&self, player_name: &str, target: &str) -> String {
        let location = self.player_location[player_name];

        match self.find_description(location, target) {
            None => "You don't see anything named like that here.\r\n".to_string(),
            Some(description) => description,
        }
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

            if mobile.name == target {
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
                return Some(format!("{} is a player. Players don't yet have a description.\r\n", player.to_pascal_case()));
            }
        }

        None
    }

    pub fn do_look(&self, player_name: &str) -> String {
        let location = self.player_location[player_name];
        let room_state = &self.rooms[location.0];
        let room = self.world.room(location);

        use std::fmt::Write;
        let mut output = String::new();

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
            write!(
                output,
                "\x1b[35m{}\x1b[0m\r\n",
                self.world.mobile(mob.vnum).long_description
            )
            .unwrap();
        }

        for player in room_state.players.keys() {
            if player == player_name {
                continue;
            }

            let proper_name = player.to_pascal_case();
            write!(output, "\x1b[1;35m{}, a player, is here.\x1b[0m\r\n", proper_name).unwrap();
        }

        output
    }

    pub fn do_move(&mut self, player_name: &str, direction: &str) -> String {
        let location = self.player_location[player_name];
        let old_room = self.world.room(location);

        let direction = match direction {
            "n" => "north",
            "e" => "east",
            "s" => "south",
            "w" => "west",
            dir => dir,
        };

        use std::fmt::Write;
        let mut output = String::new();

        if let Some(exit) = old_room.exits.iter().find(|e| e.name == direction) {
            let new_location = exit.vnum;

            if self.world.has_room(new_location) {
                let old_room_state = &mut self.rooms[location.0];
                let (name, player) = old_room_state.players.remove_entry(player_name).unwrap();
                let new_room_state = &mut self.rooms[new_location.0];
                new_room_state.players.insert(name, player);

                *self.player_location.get_mut(player_name).unwrap() = new_location;

                drop(new_room_state);

                write!(output, "You walk {}.\r\n", direction).unwrap();
                output += &self.do_look(player_name);
            } else {
                write!(output, "The way {} leads into the void!\r\n", direction).unwrap();
            }
        } else {
            write!(output, "The way to the {} is blocked.\r\n", direction).unwrap();
        }

        output
    }
}
