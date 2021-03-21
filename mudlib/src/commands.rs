use inflector::Inflector;

use crate::state::change_player_location;
use crate::world::{common_direction, long_direction, opposite_direction};
use crate::{
    echo,
    entity::{EntityId, EntityWorld},
    players::Players,
    socials::Socials,
    state::{find_description, find_target, RoomState, Target, WorldState},
    world::{Gender, Vnum, World},
    acting::EscapeVariables,
};
use crate::{entity::Found, mapper::make_map};

struct Agent<'w, 's, 'p> {
    // Info
    world: &'w World,
    socials: &'w Socials,

    // State
    players: &'p mut Players,
    rooms: &'s mut [RoomState],
}

struct EntityAgent<'e, 'p> {
    entity_world: &'e mut EntityWorld,
    players: &'p mut Players,

    entity_id: EntityId,
}

fn process_agent_command(agent: &mut EntityAgent, words: &[&str]) -> bool {
    match words {
        &["elook"] | &["el"] => {
            agent.do_look();
        }
        &["elook", target] | &["el", target] | &["elook", "at", target] | &["el", "at", target] => {
            agent.do_look_at(target);
        }
        &["force", target, ref victim_words @ ..] => {
            agent.force(target, victim_words);
        }
        &["esay", ref message @ ..] => {
            agent.do_say(&message.join(" "));
        }
        _ => return false,
    };

    true
}

pub(crate) fn process_command(world_state: &mut WorldState, words: &[&str]) {
    let world = &mut world_state.entity_world;
    let player_id = world.player_entity_id(&world_state.players.current_player);

    let player_id = match player_id {
        Some(id) => id,
        None => {
            world_state
                .players
                .current()
                .echo("But you don't seem to have a body.");
            return;
        }
    };

    let mut agent = EntityAgent {
        entity_world: &mut world_state.entity_world,
        players: &mut world_state.players,
        entity_id: player_id,
    };

    if process_agent_command(&mut agent, words) {
        return;
    }

    let mut agent = Agent {
        world: &world_state.world,
        socials: &world_state.socials,
        rooms: &mut world_state.rooms,
        players: &mut world_state.players,
    };

    use std::fmt::Write;
    match words {
        &["panic"] => {
            panic!("Oh no! I panicked!");
        }
        &["look"] | &["l"] => {
            agent.do_look();
        }
        &["look", target] | &["l", target] | &["look", "at", target] | &["l", "at", target] => {
            agent.do_look_at(target);
        }
        &["say", ref message @ ..] => {
            agent.do_say(&message.join(" "));
        }
        &["help"] => {
            let help_text = include_str!("../help.txt");

            world_state.players.current().echo(help_text);
        }
        &["map"] => {
            let location = world_state.players.locations[&world_state.players.current_player];
            let map = make_map(&world_state.world, location);
            world_state.players.current().echo(map);
        }
        &["exits"] => {
            agent.do_exits();
        }
        &["recall"] => {
            agent.do_recall(None);
        }
        &["recall", location] => {
            agent.do_recall(Some(location));
        }
        &["socials"] => {
            agent.do_socials(None);
        }
        &["socials", social] => {
            agent.do_socials(Some(social));
        }
        &["get"] => {
            agent.do_get(None);
        }
        &["get", item] => {
            agent.do_get(Some(item));
        }
        &["drop"] => {
            agent.do_drop(None);
        }
        &["drop", item] => {
            agent.do_drop(Some(item));
        }
        &["i"] | &["inv"] | &["inventory"] => {
            agent.do_inventory();
        }
        &[direction] if agent.do_move(direction) => (),
        &[social] if agent.do_act(social, None) => (),
        &[social, target] if agent.do_act(social, Some(target)) => (),
        &[cmd_word, ..] => {
            write!(
                world_state.players.current(),
                "Unrecognized command: {}. Type 'help' for a list of commands.\r\n",
                cmd_word
            )
            .unwrap();
        }
        &[] => (),
    }
}

impl<'e, 'p> EntityAgent<'e, 'p> {
    fn do_look(&mut self) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let room_id = self.entity_world.room_of(self.entity_id);
        let room = self.entity_world.entity_info(room_id);

        let mut act = self.players.act_alone(&myself);
        let mut out = act.myself();

        // Title
        echo!(out, "\x1b[33m{}\x1b[0m\r\n", room.internal_title());

        // Description
        let description = room.internal_description();
        echo!(out, "{}", description);
        if !description.ends_with("\r") && !description.ends_with("\n") {
            echo!(out, "\r\n");
        }

        // Exits
        let mut first_exit = true;
        for exit in room.exits() {
            if first_exit {
                first_exit = false;
                echo!(out, "\x1b[32mYou see exits: ");
            } else {
                echo!(out, ", ");
            }

            echo!(out, "{}", exit.keyword());
        }

        if first_exit {
            echo!(out, "\x1b[32mYou see no exits.\x1b[0m\r\n");
        } else {
            echo!(out, ".\x1b[0m\r\n");
        }

        // Objects
        for object in room.objects() {
            echo!(out, "\x1b[36m{}\x1b[0m\r\n", object.lateral_description());
        }

        // Mobiles
        for mobile in room.mobiles() {
            echo!(out, "\x1b[35m{}\x1b[0m\r\n", mobile.lateral_description());
        }

        // Players
        for player in room.players() {
            if player.entity_id() == self.entity_id {
                continue;
            }

            echo!(out, "\x1b[1;35m{}\x1b[0m\r\n", player.lateral_description());
        }
    }

    pub fn do_look_at(&mut self, target: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let target = myself.find_entity(target, |_entity| true);

        let mut output = self.players.current();

        let description = match target {
            Found::Myself => {
                "It's you! But you're sadly just a player and players don't have descriptions yet."
            }
            Found::Other(entity) => entity.external_description(),
            Found::WrongSelf => {
                unreachable!("The matcher accepts everything");
            }
            Found::WrongOther(_) => {
                unreachable!("The matcher accepts everything");
            }
            Found::Nothing => "You don't see anything named like that here.",
        };

        output.echo(description);

        if !description.ends_with("\r") && !description.ends_with("\n") {
            output.echo("\r\n");
        }
    }

    pub fn force(&mut self, target_name: &str, words: &[&str]) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let target = myself.find_entity(target_name, |_entity| true);

        let target = match target {
            Found::Myself => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "You snap your fingers at yourself.\r\n");
                echo!(act.others(), "He snaps his fingers at himself.\r\n");
                myself
            }
            Found::Other(other) => {
                let mut act = self.players.act_with(&myself, &other);
                echo!(act.myself(), "You snap your fingers at $N.\r\n");
                echo!(act.target(), "$^$n snaps $s fingers at you.\r\n");
                echo!(act.others(), "$^$n snaps $s fingers at $N.\r\n");
                other
            }
            Found::WrongSelf => {
                unreachable!("The matcher accepts everything");
            }
            Found::WrongOther(_) => {
                unreachable!("The matcher accepts everything");
            }
            Found::Nothing => {
                let mut act = self.players.act_alone(&myself);
                echo!(
                    act.myself(),
                    "You don't see anything named {} in the room.\r\n",
                    target_name
                );
                return;
            }
        };

        let mut act = self.players.act_alone(&target);
        echo!(act.myself(), "You feel compelled to: {:?}\r\n", words);

        let target_id = target.entity_id();

        let mut agent = EntityAgent {
            entity_world: self.entity_world,
            players: self.players,
            entity_id: target_id,
        };

        if !process_agent_command(&mut agent, words) {
            let myself = self.entity_world.entity_info(self.entity_id);
            let target = self.entity_world.entity_info(target_id);
            let mut act = self.players.act_with(&myself, &target);
            echo!(act.myself(), "$^$E didn't quite understand your command.\r\n");
        }
    }

    pub fn do_say(&mut self, message: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);

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

            echo!(
                act.myself(),
                "\x1b[1;35mYou say, '{}{}{}'\x1b[0m\r\n",
                uppercase_character, EscapeVariables(remaining_characters), suffix
            );

            echo!(
                act.others(),
                "\x1b[1;35m$^$n says, '{}{}{}'\x1b[0m\r\n",
                uppercase_character, EscapeVariables(remaining_characters), suffix
            );
        } else {
            echo!(act.myself(), "You say nothing whatsoever.\r\n");
        }
    }
}

impl<'w, 's, 'p> Agent<'w, 's, 'p> {
    pub fn do_look_at(&mut self, target: &str) {
        let description = find_description(self.players, self.rooms, self.world, target);

        let description = match description {
            None => "You don't see anything named like that here.\r\n".to_string(),
            Some(description) => description,
        };

        self.players.current().echo(description);
    }

    pub fn do_look(&mut self) {
        let location = self.players.locations[&self.players.current_player];
        let room_state = &self.rooms[location.0];
        let room = self.world.room(location);

        use std::fmt::Write;
        let mut output = self.players.current();

        echo!(output, "\x1b[33m{}\x1b[0m\r\n", room.name);
        echo!(output, "{}", room.description);

        if room.exits.is_empty() {
            echo!(output, "\x1b[32mYou see no exits.\x1b[0m\r\n");
        } else {
            echo!(output, "\x1b[32mYou see exits: ");
            let mut first = true;

            for exit in room.exits.iter() {
                if first {
                    first = false;
                } else {
                    echo!(output, ", ");
                }
                echo!(output, "{}", exit.name);
            }
            echo!(output, ".\x1b[0m\r\n");
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

                change_player_location(&mut self.rooms[..], &mut self.players, new_location);

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
            let target = find_target(&self.players, &self.rooms[..], &self.world, target);
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
                Target::Mobile(mobile, _state) => {
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

    pub fn do_inventory(&mut self) {
        let location = self.players.locations[&self.players.current_player];
        let room = &self.rooms[location.0];
        let player = &room.players[&self.players.current_player];

        let mut output = self.players.current();
        output.echo("You are holding:\r\n    ");
        let mut first = true;
        let mut column = 4;
        for item in &player.character.inventory {
            if first {
                first = false;
            } else {
                output.echo(", ");
                column += 2;
            }

            if column > 72 {
                output.echo("\r\n    ");
                column = 4;
            }

            let object = self.world.object(item.vnum);
            output.echo(&object.short_description);
            column += object.short_description.len();
        }
        output.echo("\r\n");
    }

    pub fn do_get(&mut self, object_name: Option<&str>) {
        let object_name = match object_name {
            Some(name) => name,
            None => {
                self.players.current().echo("Get what?\r\n");
                return;
            }
        };

        let location = self.players.locations[&self.players.current_player];
        let room = &mut self.rooms[location.0];
        let world = &self.world;

        let object_index = room.objects.iter().position(|obj| {
            let object = world.object(obj.vnum);
            object
                .name
                .split_whitespace()
                .any(|word| word == object_name)
        });
        let object_index = match object_index {
            Some(index) => index,
            None => {
                self.players
                    .current()
                    .echo("You don't see anything named like that in the room.\r\n");
                return;
            }
        };
        let object_state = room.objects.remove(object_index);
        let object = self.world.object(object_state.vnum);
        let player = room.players.get_mut(&self.players.current_player).unwrap();
        player.character.inventory.push(object_state);
        let player = self.players.current_player.to_title_case();
        self.players
            .current()
            .echo(&format!("You pick up {}.\r\n", object.short_description));
        self.players.others().echo(&format!(
            "{} picks up {}.\r\n",
            player, object.short_description
        ));
    }

    pub fn do_drop(&mut self, object_name: Option<&str>) {
        let object_name = match object_name {
            Some(name) => name,
            None => {
                self.players.current().echo("Drop what?\r\n");
                return;
            }
        };

        let location = self.players.locations[&self.players.current_player];
        let room = &mut self.rooms[location.0];
        let world = &self.world;
        let player_state = room.players.get_mut(&self.players.current_player).unwrap();

        let object_index = player_state.character.inventory.iter().position(|obj| {
            let object = world.object(obj.vnum);
            object
                .name
                .split_whitespace()
                .any(|word| word == object_name)
        });
        let object_index = match object_index {
            Some(index) => index,
            None => {
                self.players
                    .current()
                    .echo("You aren't holding anything named like that.\r\n");
                return;
            }
        };
        let object_state = player_state.character.inventory.remove(object_index);
        let vnum = object_state.vnum;
        room.objects.push(object_state);

        let object = self.world.object(vnum);
        let player = self.players.current_player.to_title_case();
        self.players
            .current()
            .echo(&format!("You drop {}.\r\n", object.short_description));
        self.players.others().echo(&format!(
            "{} drops {}.\r\n",
            player, object.short_description
        ));
    }
}
