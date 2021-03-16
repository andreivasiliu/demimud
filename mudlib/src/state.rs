use std::collections::BTreeMap;

use inflector::Inflector;

use crate::{
    players::Players,
    socials::Socials,
    world::{Exit, ExtraDescription, Gender, Mobile, Object, ResetCommand, Room, Vnum, World},
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
    pub(crate) world: World,
    pub(crate) socials: Socials,

    pub(crate) players: Players,
    rooms: Vec<RoomState>,
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
            self.rooms[23611]
                .players
                .insert(name.clone(), PlayerState {});
            self.players.locations.insert(name.clone(), Vnum(23611));
        }
        if !self.players.echoes.contains_key(&name) {
            self.players.echoes.insert(name, String::new());
        }
    }

    pub fn do_look_at(&mut self, target: &str) {
        let description = match self.find_description(target) {
            None => "You don't see anything named like that here.\r\n".to_string(),
            Some(description) => description,
        };

        self.players.current().echo(description);
    }

    pub fn find_description(&self, target: &str) -> Option<String> {
        let target = find_target(&self.players, &self.rooms, &self.world, &target);

        match target {
            Target::Me => {
                return Some("It's you! But you're sadly just a player and players don't have descriptions yet.\r\n".to_string());
            }
            Target::Exit(exit) => {
                if let Some(description) = &exit.description {
                    return Some(description.clone());
                } else {
                    return Some(format!(
                        "You don't see anything special to the {}.\r\n",
                        &exit.name
                    ));
                }
            }
            Target::Object(object) => {
                return Some(object.description.clone() + "\r\n");
            }
            Target::Mobile(mobile) => {
                return Some(mobile.description.clone());
            }
            Target::Player(player) => {
                return Some(format!(
                    "{} is a player. Players don't yet have a description.\r\n",
                    player.to_pascal_case()
                ));
            }
            Target::ObjectExtraDescription(_object, extra_description) => {
                return Some(extra_description.description.clone());
            }
            Target::RoomExtraDescription(_room, extra_description) => {
                return Some(extra_description.description.clone());
            }
            Target::NotFound(_target) => {
                return None;
            }
        }
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

    pub fn do_exits(&mut self) {
        let location = self.players.locations[&self.players.current_player];
        let room = self.world.room(location);

        let mut output = self.players.current();

        use std::fmt::Write;
        output.echo("You see the following exits:\r\n");
        for exit in &room.exits {
            let other_room = if self.world.has_room(exit.vnum) {
                &self.world.room(exit.vnum).name
            } else {
                "`DThe Void`^"
            };
            write!(
                output,
                "`W{}`^: leading to {} (v{})\r\n",
                exit.name, other_room, exit.vnum.0
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
                let player = self.players.current_player.to_title_case();

                write!(self.players.current(), "You walk {}.\r\n", direction).unwrap();
                write!(
                    self.players.others(),
                    "{} leaves {}.\r\n",
                    player,
                    direction
                )
                .unwrap();

                change_player_location(&mut self.rooms, &mut self.players, new_location);

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

    pub(crate) fn do_recall(&mut self, location: Option<&str>) {
        let mut output = self.players.current();

        let location = match location {
            Some(location) => location,
            None => {
                output.echo("You can recall to:\r\n");
                output.echo(" `Wrecall mekali`^ - A Large Plaza in Mekali City\r\n");
                output.echo(" `Wrecall gnomehill`^ - A Large Plaza on Gnome Hill\r\n");
                output.echo(" `Wrecall dzagari`^ - The Blasted Square in Dzagari\r\n");
                return;
            }
        };

        let new_location = match location {
            "mekali" => Vnum(3000),
            "gnomehill" => Vnum(23611),
            "dzagari" => Vnum(27003),
            _ => {
                output.echo("Unknown location; type `Wrecall`^ to see a list.\r\n");
                return;
            }
        };

        output.echo(
            "You close your eyes in prayer, and feel your surroundings shift around you.\r\n",
        );
        change_player_location(&mut self.rooms, &mut self.players, new_location);

        self.do_look();
    }

    pub(crate) fn do_socials(&mut self, social: Option<&str>) {
        let mut output = self.players.current();

        if let Some(social) = social {
            use std::fmt::Write;
            if let Some(social) = self.socials.get(social) {
                write!(
                    output,
                    "The {} social shows the following messages:\r\n",
                    social.name
                )
                .unwrap();
                write!(output, "Untargetted:\r\n").unwrap();
                write!(output, "  \"{}\"\r\n", social.untargetted_self).unwrap();
                write!(output, "  \"{}\"\r\n", social.untargetted_others).unwrap();

                write!(output, "Targetted:\r\n").unwrap();
                write!(output, "  \"{}\"\r\n", social.targetted_self).unwrap();
                write!(output, "  \"{}\"\r\n", social.targetted_target).unwrap();
                write!(output, "  \"{}\"\r\n", social.targetted_others).unwrap();

                write!(output, "Self-targetted:\r\n").unwrap();
                write!(output, "  \"{}\"\r\n", social.reflected_self).unwrap();
                write!(output, "  \"{}\"\r\n", social.reflected_others).unwrap();
            } else {
                output.echo("There is no social with that name.\r\n");
            }
        } else {
            output.echo("The following emotes are available:\r\n");

            let mut column = 0;
            let mut first = true;

            for social in self.socials.list() {
                if first {
                    first = false;
                } else {
                    output.echo(", ");
                    column += 2;

                    if column > 70 {
                        output.echo("\r\n");
                        column = 0;
                    }
                }

                output.echo("`W");
                output.echo(social);
                output.echo("`^");
                column += social.len();
            }

            output.echo(".\r\n");
        }
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

    pub fn do_act(&mut self, social: &str, target: Option<&str>) -> bool {
        let social = match self.socials.get(social) {
            Some(social) => social,
            None => return false,
        };

        let player = self.players.current_player.to_title_case();

        if let Some(target) = target {
            let target = find_target(&self.players, &self.rooms, &self.world, target);
            let player_title;
            let mut player_target = None;
            let objective_pronoun;
            let possessive_pronoun;
            let mut targetted_self = false;

            let target_name: &str = match target {
                Target::Me => {
                    let player = &self.players.current_player;
                    player_target = Some(player.to_string());
                    targetted_self = true;
                    objective_pronoun = "him"; // FIXME
                    possessive_pronoun = "his"; // FIXME
                    player_title = player.to_title_case();
                    &player_title
                }
                Target::Exit(exit) => {
                    objective_pronoun = "it";
                    possessive_pronoun = "its";
                    // FIXME: Short description table
                    &exit.name
                }
                Target::Object(object) => {
                    objective_pronoun = "it";
                    possessive_pronoun = "its";
                    &object.short_description
                }
                Target::Mobile(mobile) => {
                    objective_pronoun = match mobile.gender {
                        Gender::Male => "him",
                        Gender::Female => "her",
                        Gender::Neutral => "them",
                    };
                    possessive_pronoun = match mobile.gender {
                        Gender::Male => "his",
                        Gender::Female => "her",
                        Gender::Neutral => "its",
                    };
                    &mobile.short_description
                }
                Target::Player(player) => {
                    player_target = Some(player.to_string());
                    objective_pronoun = "him"; // FIXME
                    possessive_pronoun = "his"; // FIXME
                    player_title = player.to_title_case();
                    &player_title
                }
                Target::ObjectExtraDescription(_object, extra_description) => {
                    objective_pronoun = "it";
                    possessive_pronoun = "its";
                    // FIXME: Is this okay?
                    &extra_description.keyword
                }
                Target::RoomExtraDescription(_room, extra_description) => {
                    objective_pronoun = "it";
                    possessive_pronoun = "its";
                    &extra_description.keyword
                }
                Target::NotFound(target) => {
                    use std::fmt::Write;
                    let target = target.to_string();
                    write!(
                        self.players.current(),
                        "You don't see anything named '{}' here.\r\n",
                        target
                    )
                    .unwrap();
                    return true;
                }
            };

            let self_objective_pronoun = "him"; // FIXME
            let self_possessive_pronoun = "his"; // FIXME

            let replace_names = |message: &str| {
                message
                    .replace("$n", &player)
                    .replace("$m", self_objective_pronoun)
                    .replace("$s", self_possessive_pronoun)
                    .replace("$N", target_name)
                    .replace("$M", objective_pronoun)
                    .replace("$S", possessive_pronoun)
                    + "\r\n"
            };

            if targetted_self {
                let message_to_self = replace_names(&social.reflected_self);
                let message_to_others = replace_names(&social.reflected_others);

                self.players.current().echo(&message_to_self);
                self.players.others().echo(&message_to_others);
            } else {
                let message_to_self = replace_names(&social.targetted_self);
                let message_to_target = replace_names(&social.targetted_target);
                let message_to_others = replace_names(&social.targetted_others);

                self.players.current_target = player_target;

                self.players.current().echo(&message_to_self);
                self.players.target().echo(&message_to_target);
                self.players.others().echo(&message_to_others);

                self.players.current_target = None;
            }
        } else {
            let self_objective_pronoun = "him"; // FIXME
            let self_possessive_pronoun = "his"; // FIXME

            let replace_names = |message: &str| {
                message
                    .replace("$n", &player)
                    .replace("$m", self_objective_pronoun)
                    .replace("$s", self_possessive_pronoun)
                    + "\r\n"
            };

            let message_to_self = replace_names(&social.untargetted_self);
            let message_to_others = replace_names(&social.untargetted_others);

            self.players.current().echo(&message_to_self);

            if !social.untargetted_others.is_empty() {
                self.players.others().echo(&message_to_others);
            }
        }

        true
    }
}

enum Target<'a> {
    Me,
    Exit(&'a Exit),
    Object(&'a Object),
    Mobile(&'a Mobile),
    Player(&'a str),
    ObjectExtraDescription(&'a Object, &'a ExtraDescription),
    RoomExtraDescription(&'a Room, &'a ExtraDescription),
    NotFound(&'a str),
}

fn find_target<'a>(
    players: &'a Players,
    rooms: &'a Vec<RoomState>,
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
                .find(|&word| word == target)
                .is_some()
            {
                return Target::ObjectExtraDescription(&object, &extra_description);
            }
        }

        if object
            .name
            .split_whitespace()
            .find(|&word| word == target)
            .is_some()
        {
            return Target::Object(&object);
        }
    }

    for mobile_state in &room_state.mobiles {
        let mobile = world.mobile(mobile_state.vnum);

        if mobile
            .name
            .split_whitespace()
            .find(|&word| word == target)
            .is_some()
        {
            return Target::Mobile(&mobile);
        }
    }

    for extra_description in &room.extra_descriptions {
        if extra_description
            .keyword
            .split_whitespace()
            .find(|&word| word == target)
            .is_some()
        {
            return Target::RoomExtraDescription(&room, &extra_description);
        }
    }

    for player in room_state.players.keys() {
        if player == target {
            return Target::Player(player);
        }
    }

    return Target::NotFound(target);
}

fn change_player_location(
    rooms: &mut Vec<RoomState>,
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
