use string_interner::StringInterner;

use crate::world::{common_direction, long_direction, opposite_direction};
use crate::{
    acting::EscapeVariables,
    acting::Players,
    echo,
    entity::{EntityId, EntityWorld},
    socials::Socials,
    state::WorldState,
};
use crate::{entity::Found, mapper::make_map};

struct EntityAgent<'e, 'p> {
    entity_world: &'e mut EntityWorld,
    socials: &'e Socials,
    players: &'p mut Players,

    entity_id: EntityId,
}

fn process_agent_command(agent: &mut EntityAgent, words: &[&str]) -> bool {
    match words {
        &["panic"] => {
            panic!("Oh no! I panicked!");
        }
        &["help"] => {
            agent.do_help();
        }
        &["map"] => {
            agent.do_map();
        }
        &["look"] | &["l"] => {
            agent.do_look();
        }
        &["look", target] | &["l", target] | &["look", "at", target] | &["l", "at", target] => {
            agent.do_look_at(target);
        }
        &["force", target, ref victim_words @ ..] => {
            agent.force(target, victim_words);
        }
        &["say", ref message @ ..] => {
            agent.do_say(&message.join(" "));
        }
        &["recall"] => {
            agent.do_recall(None);
        }
        &["recall", location] => {
            agent.do_recall(Some(location));
        }
        &["exits"] => {
            agent.do_exits();
        }
        &["get"] => {
            agent.do_get(None, false);
        }
        &["get", item] => {
            agent.do_get(Some(item), false);
        }
        &["get", item, "forcefully"] => {
            agent.do_get(Some(item), true);
        }
        &["drop"] => {
            agent.do_drop(None, false);
        }
        &["drop", item] => {
            agent.do_drop(Some(item), false);
        }
        &["drop", item, "forcefully"] => {
            agent.do_drop(Some(item), true);
        }
        &["i"] | &["inv"] | &["inventory"] => {
            agent.do_inventory();
        }
        &["socials"] | &["emotes"] => {
            agent.do_socials(None);
        }
        &["socials", social] | &["emotes", social] => {
            agent.do_socials(Some(social));
        }
        &[direction] if agent.do_move(direction) => (),
        &[social] if agent.do_social(social, None) => (),
        &[social, target] if agent.do_social(social, Some(target)) => (),
        &[cmd_word, ..] => {
            agent.do_unknown(cmd_word);
            return false;
        }
        &[] => (),
    };

    true
}

pub(crate) fn process_player_command(world_state: &mut WorldState, player: &str, words: &[&str]) {
    let world = &mut world_state.entity_world;
    let player_id = world.player_entity_id(player);

    let player_id = match player_id {
        Some(id) => id,
        None => {
            if let Some(player_echo) = world_state.players.player_echoes.get_mut(player) {
                player_echo
                    .echo_buffer
                    .push_str("But you don't seem to have a body.");
            }
            return;
        }
    };

    let mut agent = EntityAgent {
        entity_world: &mut world_state.entity_world,
        socials: &world_state.socials,
        players: &mut world_state.players,
        entity_id: player_id,
    };

    process_agent_command(&mut agent, words);
}

impl<'e, 'p> EntityAgent<'e, 'p> {
    fn do_unknown(&mut self, cmd_word: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);

        echo!(
            act.myself(),
            "Unrecognized command: {}. Type 'help' for a list of commands.\r\n",
            EscapeVariables(cmd_word)
        );
    }

    fn do_help(&mut self) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);

        let help_text = include_str!("../help.txt");
        echo!(act.myself(), "{}", help_text);
    }

    fn do_map(&mut self) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let map = make_map(
            &self.entity_world,
            self.entity_world.room_of(self.entity_id),
        );

        let mut act = self.players.act_alone(&myself);
        echo!(act.myself(), "{}", map);
    }

    fn do_look(&mut self) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let room_id = self.entity_world.room_of(self.entity_id);
        let room = self.entity_world.entity_info(room_id);

        let mut act = self.players.act_alone(&myself);
        let mut out = act.myself();

        // Title
        echo!(
            out,
            "\x1b[33m{}\x1b[0m\r\n",
            room.component_info().internal_title()
        );

        // Description
        let description = room.component_info().internal_description();
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

            echo!(out, "{}", exit.component_info().keyword());
        }

        if first_exit {
            echo!(out, "\x1b[32mYou see no exits.\x1b[0m\r\n");
        } else {
            echo!(out, ".\x1b[0m\r\n");
        }

        // Objects
        for object in room.objects() {
            echo!(
                out,
                "\x1b[36m{}\x1b[0m\r\n",
                object.component_info().lateral_description()
            );
        }

        // Mobiles
        for mobile in room.mobiles() {
            echo!(
                out,
                "\x1b[35m{}\x1b[0m\r\n",
                mobile.component_info().lateral_description()
            );
        }

        // Players
        for player in room.players() {
            if player.entity_id() == self.entity_id {
                continue;
            }

            echo!(
                out,
                "\x1b[1;35m{}\x1b[0m\r\n",
                player.component_info().lateral_description()
            );
        }
    }

    pub fn do_look_at(&mut self, target: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let target = myself.find_entity(target, |_entity| true);

        let mut act = self.players.act_alone(&myself);

        let target = match target {
            Found::Myself => {
                echo!(act.myself(), "It's you! But you're sadly just a player and players don't have descriptions yet.\r\n");
                return;
            }
            Found::Other(entity) => entity,
            Found::WrongSelf => {
                unreachable!("The matcher accepts everything");
            }
            Found::WrongOther(_) => {
                unreachable!("The matcher accepts everything");
            }
            Found::Nothing => {
                echo!(
                    act.myself(),
                    "You don't see anything named like that here.\r\n"
                );
                return;
            }
        };

        // Description
        let description = target.component_info().external_description();
        let newline = if description.ends_with("\r") || description.ends_with("\n") {
            ""
        } else {
            "\r\n"
        };

        echo!(act.myself(), "{}{}", description, newline);

        let mut act = self.players.act_with(&myself, &target);

        if !target.is_extra_description() {
            echo!(act.target(), "$^$n looks at you.\r\n");
            echo!(act.others(), "$^$n looks at $N.\r\n");
        }

        // Contents
        let mut first = true;
        for item in target.contained_entities() {
            if item.equipped().is_none() {
                if first {
                    echo!(act.myself(), "$^$E is holding:\r\n    ");
                    first = false;
                } else {
                    echo!(act.myself(), ", ");
                }
                echo!(
                    act.myself(),
                    "{}",
                    item.component_info().short_description()
                );
            }
        }
        if !first {
            echo!(act.myself(), "\r\n");
        }

        // Equipment
        let mut first = true;
        for item in target.contained_entities() {
            if let Some(location) = item.equipped() {
                if first {
                    echo!(act.myself(), "$^$E is wearing:\r\n");
                    first = false;
                }
                echo!(
                    act.myself(),
                    "  `S[`y{}`S]:`^ {}\r\n",
                    location,
                    item.component_info().short_description()
                );
            }
        }
    }

    pub fn force(&mut self, target_name: &str, words: &[&str]) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let target = myself.find_entity(target_name, |entity| {
            // Prefer objects over their descriptions
            !entity.is_extra_description()
        });

        let target = match target {
            Found::Myself | Found::WrongSelf => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "You snap your fingers at yourself.\r\n");
                echo!(act.others(), "He snaps his fingers at himself.\r\n");
                myself
            }
            Found::Other(other) | Found::WrongOther(other) => {
                let mut act = self.players.act_with(&myself, &other);
                echo!(act.myself(), "You snap your fingers at $N.\r\n");
                echo!(act.target(), "$^$n snaps $s fingers at you.\r\n");
                echo!(act.others(), "$^$n snaps $s fingers at $N.\r\n");
                other
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
            socials: self.socials,
            players: self.players,
            entity_id: target_id,
        };

        if !process_agent_command(&mut agent, words) {
            let myself = self.entity_world.entity_info(self.entity_id);
            let target = self.entity_world.entity_info(target_id);
            let mut act = self.players.act_with(&myself, &target);
            echo!(
                act.myself(),
                "$^$E didn't quite understand your command.\r\n"
            );
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
                uppercase_character,
                EscapeVariables(remaining_characters),
                suffix
            );

            echo!(
                act.others(),
                "\x1b[1;35m$^$n says, '{}{}{}'\x1b[0m\r\n",
                uppercase_character,
                EscapeVariables(remaining_characters),
                suffix
            );
        } else {
            echo!(act.myself(), "You say nothing whatsoever.\r\n");
        }
    }

    pub(crate) fn do_recall(&mut self, location: Option<&str>) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);

        let location = match location {
            Some(location) => location,
            None => {
                echo!(act.myself(), "You can recall to:\r\n");
                echo!(
                    act.myself(),
                    " `Wrecall mekali`^ - A Large Plaza in Mekali City\r\n"
                );
                echo!(
                    act.myself(),
                    " `Wrecall gnomehill`^ - A Large Plaza on Gnome Hill\r\n"
                );
                echo!(
                    act.myself(),
                    " `Wrecall dzagari`^ - The Blasted Square in Dzagari\r\n"
                );
                return;
            }
        };

        if let Some(room_id) = self.entity_world.landmark(location) {
            echo!(
                act.myself(),
                "You close your eyes in prayer, and feel your surroundings shift around you.\r\n",
            );

            self.entity_world.move_entity(self.entity_id, room_id);
            self.do_look();
        } else {
            echo!(
                act.myself(),
                "Unknown location; type `Wrecall`^ to see a list.\r\n"
            );
        }
    }

    pub fn do_move(&mut self, direction: &str) -> bool {
        let direction = long_direction(direction);

        let myself = self.entity_world.entity_info(self.entity_id);
        let target = myself.find_entity(direction, |entity| entity.is_exit());

        let mut act = self.players.act_alone(&myself);

        let exit = match target {
            Found::Myself => {
                // Not hard to implement, but not worth the effort.
                echo!(
                    act.myself(),
                    "That's you! And.. wait, you're a valid exit to go into? But still, no.\r\n"
                );
                return true;
            }
            Found::Other(exit) => exit,
            Found::WrongSelf => {
                echo!(
                    act.myself(),
                    "That's you! And you're not an exit to go in.\r\n"
                );
                return true;
            }
            Found::WrongOther(target) => {
                let mut act = self.players.act_with(&myself, &target);
                echo!(act.myself(), "$^$n is not an exit you go in.\r\n");
                return true;
            }
            Found::Nothing => {
                if common_direction(direction) {
                    echo!(act.myself(), "The way to the {} is blocked.\r\n", direction);
                    return true;
                } else {
                    return false;
                }
            }
        };

        if let Some(to_room_id) = exit.leads_to() {
            let exit_id = exit.entity_id();

            echo!(
                act.myself(),
                "You walk {}.\r\n",
                exit.component_info().keyword()
            );
            echo!(
                act.others(),
                "$^$n leaves {}.\r\n",
                exit.component_info().keyword()
            );
            self.entity_world.move_entity(self.entity_id, to_room_id);

            // Reacquire everything, the acting stage is now changed.
            let myself = self.entity_world.entity_info(self.entity_id);
            let exit = self.entity_world.entity_info(exit_id);
            let mut act = self.players.act_alone(&myself);
            echo!(
                act.others(),
                "$^$n arrives from the {}.\r\n",
                opposite_direction(exit.component_info().keyword())
            );

            // Admire new surroundings.
            self.do_look();
        } else {
            echo!(act.myself(), "That exit leads into the void!\r\n")
        }

        true
    }

    pub fn do_exits(&mut self) {
        let room_id = self.entity_world.room_of(self.entity_id);
        let room = self.entity_world.entity_info(room_id);

        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);

        echo!(act.myself(), "You see the following exits:\r\n");
        for exit in room.exits() {
            let other_room = if let Some(leads_to) = exit.leads_to() {
                self.entity_world
                    .entity_info(leads_to)
                    .component_info()
                    .internal_title()
            } else {
                "`DThe Void`^"
            };

            echo!(
                act.myself(),
                "  `W{}`^: {} leading to `y{}`^.\r\n",
                exit.component_info().keyword(),
                exit.component_info().short_description(),
                other_room
            )
        }
    }

    pub fn do_socials(&mut self, social: Option<&str>) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);
        let mut me = act.myself();

        if let Some(social) = social {
            if let Some(social) = self.socials.get(social) {
                echo!(
                    me,
                    "The {} emote shows the following messages:\r\n",
                    social.name
                );
                echo!(me, "Untargetted:\r\n");
                echo!(
                    me,
                    "  \"{}\"\r\n",
                    EscapeVariables(&social.untargetted_self)
                );
                echo!(
                    me,
                    "  \"{}\"\r\n",
                    EscapeVariables(&social.untargetted_others)
                );

                echo!(me, "Targetted:\r\n");
                echo!(me, "  \"{}\"\r\n", EscapeVariables(&social.targetted_self));
                echo!(
                    me,
                    "  \"{}\"\r\n",
                    EscapeVariables(&social.targetted_target)
                );
                echo!(
                    me,
                    "  \"{}\"\r\n",
                    EscapeVariables(&social.targetted_others)
                );

                echo!(me, "Self-targetted:\r\n");
                echo!(me, "  \"{}\"\r\n", EscapeVariables(&social.reflected_self));
                echo!(
                    me,
                    "  \"{}\"\r\n",
                    EscapeVariables(&social.reflected_others)
                );
            } else {
                echo!(me, "There is no social with that name.\r\n");
            }
        } else {
            echo!(me, "The following emotes are available:\r\n");

            let mut column = 0;
            let mut first = true;

            for social in self.socials.list() {
                if first {
                    first = false;
                } else {
                    echo!(me, ", ");
                    column += 2;

                    if column > 70 {
                        echo!(me, "\r\n");
                        column = 0;
                    }
                }

                echo!(me, "`W{}`^", social);
                column += social.len();
            }

            echo!(me, ".\r\n");
        }
    }

    pub fn do_social(&mut self, social: &str, target: Option<&str>) -> bool {
        let social = match self.socials.get(social) {
            Some(social) => social,
            None => return false,
        };

        let myself = self.entity_world.entity_info(self.entity_id);

        if let Some(target) = target {
            let target = myself.find_entity(target, |e| !e.is_extra_description());

            match target {
                Found::Myself | Found::WrongSelf => {
                    let mut act = self.players.act_alone(&myself);
                    echo!(act.myself(), "{}\r\n", &social.reflected_self);
                    if !social.reflected_others.is_empty() {
                        echo!(act.others(), "{}\r\n", &social.reflected_others);
                    }
                }
                Found::Other(target) => {
                    let mut act = self.players.act_with(&myself, &target);
                    echo!(act.myself(), "{}\r\n", &social.targetted_self);

                    if !social.targetted_target.is_empty() {
                        echo!(act.target(), "{}\r\n", &social.targetted_target);
                    }
                    if !social.targetted_others.is_empty() {
                        echo!(act.others(), "{}\r\n", &social.targetted_others);
                    }
                }
                Found::WrongOther(other) => {
                    // Technically there's nothing wrong with emoting to extra
                    // descriptions, but the area data from Dawn of Time doesn't
                    // have good short descriptions to refer to them.
                    let mut act = self.players.act_with(&myself, &other);
                    echo!(
                        act.myself(),
                        "$^$N is not something you can iteract with.\r\n"
                    );
                }
                Found::Nothing => {
                    let mut act = self.players.act_alone(&myself);
                    echo!(
                        act.myself(),
                        "You don't see anything named like that here.\r\n"
                    );
                }
            }
        } else {
            let mut act = self.players.act_alone(&myself);
            echo!(act.myself(), "{}\r\n", &social.untargetted_self);
            if !social.untargetted_others.is_empty() {
                echo!(act.others(), "{}\r\n", &social.untargetted_others);
            }
        }

        true
    }

    pub fn do_get(&mut self, object_name: Option<&str>, forcefully: bool) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let object_name = match object_name {
            Some(name) => name,
            None => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "Get what?\r\n");
                return;
            }
        };

        let room_id = self.entity_world.room_of(self.entity_id);
        let target = myself.find_entity(object_name, |object| {
            let object_room_id = self.entity_world.room_of(object.entity_id());
            (object.is_object() || forcefully) && object_room_id == room_id
        });

        match target {
            Found::Myself | Found::WrongSelf => {
                let mut act = self.players.act_alone(&myself);
                echo!(
                    act.myself(),
                    "You try to get a hold of yourself. You think you succeeded.\r\n"
                );
                return;
            }
            Found::Other(other) => {
                let mut act = self.players.act_with(&myself, &other);
                echo!(act.myself(), "You pick up $N.\r\n");
                echo!(
                    act.target(),
                    "$^$n picks you up. You're now in $s inventory!\r\n"
                );
                echo!(act.others(), "$^$n picks up $N.\r\n");

                let myself_id = myself.entity_id();
                let other_id = other.entity_id();

                self.entity_world.move_entity(other_id, myself_id);

                let other = self.entity_world.entity_info(other_id);
                let mut act = self.players.act_alone(&other);
                echo!(
                    act.others(),
                    "$^$n is tossed into here, and lands with a thud.\r\n"
                );
            }
            Found::Nothing | Found::WrongOther(_) => {
                let mut act = self.players.act_alone(&myself);
                echo!(
                    act.myself(),
                    "You don't see any objects named like that in the room.\r\n"
                );
                return;
            }
        }
    }

    pub fn do_drop(&mut self, object_name: Option<&str>, forcefully: bool) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let object_name = match object_name {
            Some(name) => name,
            None => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "Drop what?\r\n");
                return;
            }
        };

        let target = myself.find_entity(object_name, |object| object.is_object() || forcefully);

        match target {
            Found::Myself | Found::WrongSelf => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "You attempt to let go of yourself, but somehow the rest of you just keeps on\r\n");
                echo!(act.myself(), "sticking to your hand.\r\n");
                return;
            }
            Found::Other(other) => {
                let mut act = self.players.act_with(&myself, &other);
                if self.entity_world.room_of(other.entity_id()) != myself.entity_id() {
                    echo!(act.myself(), "You aren't holding $N.\r\n");
                } else {
                    echo!(act.myself(), "You drop $N.\r\n");
                    echo!(act.target(), "$^$n drops you out of $m.\r\n");
                    echo!(act.others(), "$^$n drops $N.\r\n");

                    let mut act = self.players.act_alone(&other);
                    echo!(act.others(), "$^$n is tossed out of.\r\n");

                    let other_id = other.entity_id();
                    let room_id = self.entity_world.room_of(myself.entity_id());
                    self.entity_world.move_entity(other_id, room_id);
                }
            }
            Found::Nothing | Found::WrongOther(_) => {
                let mut act = self.players.act_alone(&myself);
                echo!(
                    act.myself(),
                    "You aren't holding anything named like that.\r\n"
                );
                return;
            }
        }
    }

    pub fn do_inventory(&mut self) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let mut act = self.players.act_alone(&myself);
        echo!(act.myself(), "You are holding:\r\n    ");
        let mut first = true;
        let mut column = 4;
        for item in myself.contained_entities() {
            if first {
                first = false;
            } else {
                echo!(act.myself(), ", ");
                column += 2;
            }

            if column > 72 {
                echo!(act.myself(), "\r\n    ");
                column = 4;
            }

            echo!(
                act.myself(),
                "{}",
                EscapeVariables(item.component_info().short_description())
            );
            column += item.component_info().short_description().len();
        }
        echo!(act.myself(), "\r\n");
    }
}

pub(super) fn update_entity_world(world_state: &mut WorldState) {
    let entity_world = &mut world_state.entity_world;
    let mut interner = StringInterner::default();

    let mut wanderers = Vec::new();

    for entity in entity_world.all_entities() {
        let wander = match &entity.components().mobile {
            Some(mobile) => mobile.wander,
            None => continue,
        };

        if !wander || !random_bits(4) {
            continue;
        }

        let room_id = entity_world.room_of(entity.entity_id());
        let room = entity_world.entity_info(room_id);

        let random_exit = rand::random::<usize>() % 10;

        if let Some(exit) = room.exits().nth(random_exit) {
            let entity_id = entity.entity_id();

            let exit_symbol = interner.get_or_intern(exit.component_info().keyword());
            wanderers.push((entity_id, exit_symbol));
        }
    }

    for (wanderer_id, exit_symbol) in wanderers {
        let mut agent = EntityAgent {
            entity_world,
            socials: &world_state.socials,
            players: &mut world_state.players,
            entity_id: wanderer_id,
        };
        let exit_name = interner
            .resolve(exit_symbol)
            .expect("Interned in previous loop");

        agent.do_move(exit_name);
    }
}

fn random_bits(bits: u8) -> bool {
    (rand::random::<u32>() >> 7) & ((1u32 << bits) - 1) == 0
}
