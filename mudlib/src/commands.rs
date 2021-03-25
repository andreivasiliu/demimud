use string_interner::StringInterner;

use crate::{components::Mobile, import::VnumTemplates, world::{MobProgTrigger, common_direction, long_direction, opposite_direction}};
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
    vnum_templates: &'e VnumTemplates,
    players: &'p mut Players,

    entity_id: EntityId,
}

impl EntityAgent<'_, '_> {
    pub fn switch_agent<'a>(&'a mut self, entity_id: EntityId) -> EntityAgent<'a, 'a> {
        EntityAgent {
            entity_world: self.entity_world,
            socials: self.socials,
            vnum_templates: self.vnum_templates,
            players: self.players,

            entity_id,
        }
    }
}

fn process_agent_command(agent: &mut EntityAgent, words: &[&str]) -> bool {
    match words {
        &["panic"] => {
            panic!("Oh no! I panicked!");
        }
        &["help"] => {
            agent.do_help();
        }
        &["mq", ticks, ref command @ ..] => {
            agent.do_queue(ticks, command.join(" "));
        }
        &[mq, ref command @ ..] if mq.starts_with("mq") && mq[2..].parse::<u32>().is_ok() => {
            agent.do_queue(&mq[2..], command.join(" "));
        }
        &["mob", ref command @ ..] => {
            agent.do_mob(command);
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
            agent.do_force(target, victim_words);
        }
        &["say", target, ref message @ ..] if target.starts_with(">") => {
            agent.do_say_to(&target[1..], &message.join(" "));
        }
        &["sayto", target, ref message @ ..] => {
            agent.do_say_to(target, &message.join(" "));
        }
        &["say", ref message @ ..] => {
            agent.do_say(&message.join(" "));
        }
        &[target, ref message @ ..] if target.starts_with("'>") => {
            agent.do_say_to(&target[2..], &message.join(" "));
        }
        &[ref message @ ..] if message.len() > 0 && message[0].starts_with("'") => {
            agent.do_say(&message.join(" ")[1..]);
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
        &["list"] | &["wares"] => {
            agent.do_list();
        }
        &["emote", ref message @ ..] => {
            agent.do_emote(&message.join(" "));
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
        vnum_templates: &world_state.vnum_templates,
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

    fn do_queue(&mut self, ticks: &str, command: String) {
        let mut myself = self.entity_world.entity_info_mut(self.entity_id);
        let ticks = match ticks.parse() {
            Ok(ticks) => ticks,
            Err(_) => {
                let myself = self.entity_world.entity_info(self.entity_id);
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "The number of ticks must be a number.\r\n");
                return;
            }
        };
        myself.components().general.command_queue.push((ticks, command));

        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);
        echo!(act.myself(), "Command queued to run in {} ticks.\r\n", ticks);
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
            if let Some(door) = &exit.components().door {
                let state = if door.closed && door.locked {
                    "locked"
                } else if door.closed {
                    "closed"
                } else {
                    "open"
                };
                echo!(out, " ({})", state);
            }
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

    pub fn do_force(&mut self, target_name: &str, words: &[&str]) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let target = myself.find_entity(target_name, |entity| {
            // Prefer objects over their descriptions, and others over self
            !entity.is_extra_description() && entity.entity_id() != myself.entity_id()
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

        let mut agent = self.switch_agent(target_id);

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

            self.check_triggers(Action::Speech { message })
        } else {
            echo!(act.myself(), "You say nothing whatsoever.\r\n");
        }
    }

    pub fn do_say_to(&mut self, target: &str, message: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let target = myself.find_entity(target, |_| true);

        let target = match target {
            Found::Myself | Found::WrongSelf => {
                let myself = self.entity_world.entity_info(self.entity_id);
                let mut act = self.players.act_alone(&myself);

                // FIXME implement proper "says to $mself"
                echo!(act.myself(), "You mutter something to yourself, but nobody hears it.\r\n");
                echo!(act.others(), "$^$n mutters something to himself.\r\n");
                return;
            }
            Found::Other(other) | Found::WrongOther(other) => {
                other
            }
            Found::Nothing => {
                let myself = self.entity_world.entity_info(self.entity_id);
                let mut act = self.players.act_alone(&myself);

                echo!(act.myself(), "You don't see anyone named like that here.\r\n");
                return;
            }
        };

        let mut act = self.players.act_with(&myself, &target);

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
                "\x1b[1;35mYou say to $N, '{}{}{}'\x1b[0m\r\n",
                uppercase_character,
                EscapeVariables(remaining_characters),
                suffix
            );

            echo!(
                act.target(),
                "\x1b[1;35m$^$n says to you, '{}{}{}'\x1b[0m\r\n",
                uppercase_character,
                EscapeVariables(remaining_characters),
                suffix
            );

            echo!(
                act.others(),
                "\x1b[1;35m$^$n says to $N, '{}{}{}'\x1b[0m\r\n",
                uppercase_character,
                EscapeVariables(remaining_characters),
                suffix
            );

            self.check_triggers(Action::Speech { message })
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
                echo!(
                    act.myself(),
                    " `Wrecall mudschool`^ - The Welcome room in MudSchool\r\n"
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

            // A temporary substitute for logging in to make it easier to test
            self.check_triggers(Action::Login);
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

            // Others might admire you.
            self.check_triggers(Action::Greet);
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

    pub fn do_emote(&mut self, message: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);

        echo!(act.myself(), "You emote: $^$n {}\r\n", message);
        echo!(act.others(), "$^$n {}\r\n", message);
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

    pub fn do_list(&mut self) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let room = myself.room();

        let shopkeeper = room.contained_entities().filter_map(|entity| {
            match &entity.components().mobile {
                Some(Mobile { shopkeeper: Some(shopkeeper), .. }) => Some((entity, shopkeeper)),
                Some(_) | None => None,
            }
        }).next();

        if let Some((entity, shop_info)) = shopkeeper {
            let mut act = self.players.act_with(&myself, &entity);
            echo!(act.target(), "$^$n asks you about your wares, and you show $m what you have.\r\n");
            echo!(act.others(), "$^$n asks $N about $S wares.\r\n");

            echo!(act.myself(), "$^$N shows you $S wares:\r\n");
            for item in entity.objects() {
                if let Some(object_info) = &item.components().object {
                    let price = object_info.cost * shop_info.profit_buy as i32 / 100;
                    echo!(act.myself(), "  {}: {} silver coins\r\n", item.component_info().short_description(), price);
                } else {
                    echo!(act.myself(), "  {}: priceless\r\n", item.component_info().short_description());
                }
            }
        } else {
            let mut act = self.players.act_alone(&myself);
            echo!(act.myself(), "You don't see any shopkeepers here.\r\n");
        }
    }
}

// Mob commands
impl<'e, 'p> EntityAgent<'e, 'p> {
    pub fn do_mob(&mut self, words: &[&str]) {
        match words {
            &["transfer", target, to_room] => {
                self.do_mob_transfer(target, to_room);
            }
            &["dequeueall"] => {
                self.do_mob_dequeue_all();
            }
            &["at", room, ref command @ ..] => {
                self.do_mob_at(room, command);
            }
            &["mload", m_vnum] => {
                self.do_mob_mload(m_vnum);
            }
            &["call", p_vnum, target] => {
                self.do_call(p_vnum, target);
            }
            &["force", target, ref command @ ..] => {
                self.do_force(target, command);
            }
            &[cmd_word, ..] => {
                self.do_unknown(cmd_word);
            }
            &[] => {
                self.do_unknown("<none>");
            }
        };
    }

    pub fn do_mob_transfer(&mut self, target_name: &str, to_room: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let to_room_vnum: usize = match to_room.parse() {
            Ok(vnum) => vnum,
            Err(_) => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "Transfer room target '{}' is not a valid vnum.\r\n", to_room);
                return;
            }
        };

        let room = self
            .vnum_templates
            .vnum_to_entity
            .get(to_room_vnum)
            .and_then(|permanent_id| *permanent_id)
            .and_then(|permanent_id| self.entity_world.old_entity( &permanent_id));

        let room_id = match room {
            Some(entity) => entity.entity_id(),
            None => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "Transfer room target '{}' was destroyed.\r\n", to_room);
                return;
            }
        };

        let target = myself.find_entity(target_name, |_| true);

        let target_id = match target {
            Found::Myself | Found::WrongSelf => myself.entity_id(),
            Found::Other(other) | Found::WrongOther(other) => other.entity_id(),
            Found::Nothing => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "I don't see anyone here by that name to transfer.\r\n");
                return;
            }
        };

        let target = self.entity_world.entity_info(target_id);
        let mut act = self.players.act_with(&myself, &target);
        echo!(act.myself(), "You teleport $N to another room.\r\n");
        echo!(act.target(), "$^$n teleports you to another room.\r\n");
        echo!(act.others(), "$^$n teleports $N to another room.\r\n");

        self.entity_world.move_entity(target_id, room_id);

        let target = self.entity_world.entity_info(target_id);
        let mut act = self.players.act_alone(&target);
        echo!(act.others(), "$^$n appears into the room out of thin air.\r\n");
    }

    pub fn do_mob_dequeue_all(&mut self) {
        let mut myself = self.entity_world.entity_info_mut(self.entity_id);
        let queue = &mut myself.components().general.command_queue;

        let commands = queue.len();
        queue.clear();

        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);

        echo!(act.myself(), "Cleared {} commands from the queue.\r\n", commands);
    }

    pub fn do_mob_at(&mut self, at_room: &str, commands: &[&str]) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let at_room_vnum: usize = match at_room.parse() {
            Ok(vnum) => vnum,
            Err(_) => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "Room '{}' is not a valid vnum.\r\n", at_room);
                return;
            }
        };

        let room = self
            .vnum_templates
            .vnum_to_entity
            .get(at_room_vnum)
            .and_then(|permanent_id| *permanent_id)
            .and_then(|permanent_id| self.entity_world.old_entity( &permanent_id));

        let room_id = match room {
            Some(entity) => entity.entity_id(),
            None => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "Room '{}' was destroyed.\r\n", at_room);
                return;
            }
        };

        let original_room_id = myself.room().entity_id();

        self.entity_world.move_entity(self.entity_id, room_id);
        process_agent_command(self, commands);
        self.entity_world.move_entity(self.entity_id, original_room_id);
    }

    pub fn do_mob_mload(&mut self, m_vnum: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let m_vnum: usize = match m_vnum.parse() {
            Ok(vnum) => vnum,
            Err(_) => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "Vnum '{}' is not a valid number.\r\n", m_vnum);
                return;
            }
        };

        let mobile_components = self
            .vnum_templates
            .mobile_components
            .get(m_vnum)
            .and_then(|components| components.as_ref());

        let (mobile_components, mobprogs) = match mobile_components {
            Some(components) => components,
            None => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "Mobile template with vnum '{}' does not exist.\r\n", m_vnum);
                return;
            }
        };

        let room_id = myself.room().entity_id();

        let mobile_id = self.entity_world.insert_entity(room_id, mobile_components.clone());
        for mobprog in mobprogs {
            self.entity_world.insert_entity(mobile_id, mobprog.clone());
        }

        let myself = self.entity_world.entity_info(self.entity_id);
        let mobile = self.entity_world.entity_info(mobile_id);
        let mut act = self.players.act_with(&mobile, &myself);
        echo!(act.target(), "Spawned $N from mobile template with vnum '{}' .\r\n", m_vnum);
        echo!(act.myself(), "You have been spawned by $n into existence. Welcome!\r\n");
        echo!(act.others(), "$^$n creates $N from thin air and drops $M into the room.\r\n");
    }

    pub fn do_call(&mut self, p_vnum: &str, target: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let p_vnum: usize = match p_vnum.parse() {
            Ok(vnum) => vnum,
            Err(_) => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "Vnum '{}' is not a valid number.\r\n", p_vnum);
                return;
            }
        };

        let mobprog = self
            .vnum_templates
            .vnum_to_mobprog
            .get(p_vnum)
            .and_then(|mobprog| mobprog.as_ref());

        let code = match mobprog {
            Some(code) => code.clone(),
            None => {
                let mut act = self.players.act_alone(&myself);
                echo!(act.myself(), "MobProg with vnum '{}' does not exist.\r\n", p_vnum);
                return;
            }
        };

        self.run_mobprog(code, target.to_string());
    }
}

pub(crate) enum Action<'a> {
    Speech { message: &'a str },
    Greet,
    Login,
}

impl<'e, 'p> EntityAgent<'e, 'p> {
    fn check_triggers(&mut self, action: Action<'_>) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let room = myself.room();
        let mut triggered = Vec::new();

        if !myself.is_player() {
            return;
        }

        for entity in room.contained_entities() {
            for item in entity.contained_entities() {
                if let Some(mobprog) = &item.components().mobprog {
                    match (&action, &mobprog.trigger) {
                        (Action::Speech { message }, MobProgTrigger::Speech { pattern }) => {
                            if message.find(pattern).is_some() {
                                triggered.push((entity.entity_id(), mobprog.code.clone()));
                            }
                        }
                        (Action::Greet, MobProgTrigger::Greet { chance }) => {
                            if random_percent(*chance) {
                                triggered.push((entity.entity_id(), mobprog.code.clone()));
                            }
                        }
                        (Action::Login, MobProgTrigger::LoginRoom) => {
                            triggered.push((entity.entity_id(), mobprog.code.clone()));
                        }
                        _ => (),
                    };
                }
            }
        }

        if triggered.is_empty() {
            return;
        }

        let self_keyword = myself.main_keyword().to_string();

        for (entity_id, code) in triggered {
            let mut agent = self.switch_agent(entity_id);
            agent.run_mobprog(code, self_keyword.clone());
        }
    }

    pub fn run_mobprog(&mut self, code: String, target: String) {
        for command in code.lines().map(|c| c.replace("$n", &target)) {
            let words: Vec<_> = command.split_whitespace().collect();
            println!("Processing: {:?}", words);
            process_agent_command(self, &words);
        }
    }
}

pub(super) fn update_entity_world(world_state: &mut WorldState) {
    update_wander(world_state);
    update_command_queue(world_state);
}

pub(super) fn update_wander(world_state: &mut WorldState) {
    world_state.wander_ticks += 1;

    // Make mobs wander every 4 seconds.
    if world_state.wander_ticks < 4 {
        return;
    }

    world_state.wander_ticks = 0;

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
            vnum_templates: &world_state.vnum_templates,
            players: &mut world_state.players,
            entity_id: wanderer_id,
        };
        let exit_name = interner
            .resolve(exit_symbol)
            .expect("Interned in previous loop");

        agent.do_move(exit_name);
    }
}

pub(super) fn update_command_queue(world_state: &mut WorldState) {
    let entity_world = &mut world_state.entity_world;
    let mut commands = Vec::new();

    for mut entity in entity_world.all_entities_mut() {
        let entity_id = entity.entity_id();
        let command_queue = &mut entity.components().general.command_queue;

        if command_queue.is_empty() {
            continue;
        }

        for (tick, _command) in command_queue.iter_mut() {
            *tick = tick.saturating_sub(1);
        }

        command_queue.retain(|(tick, command)| {
            if *tick == 0 {
                // Not ideal; how do I get the original string out without cloning it?
                // Maybe .drain_filter, but it's not stable yet
                commands.push((entity_id, command.clone()));
                false
            } else {
                true
            }
        });
    }

    for (entity_id, command) in commands {
        let mut agent = EntityAgent {
            entity_world,
            socials: &world_state.socials,
            vnum_templates: &world_state.vnum_templates,
            players: &mut world_state.players,
            entity_id,
        };

        let command_words: Vec<_> = command.split_whitespace().collect();
        process_agent_command(&mut agent, &command_words);
    }
}

fn random_bits(bits: u8) -> bool {
    (rand::random::<u32>() >> 7) & ((1u32 << bits) - 1) == 0
}

fn random_percent(chance: u8) -> bool {
    rand::random::<u32>() % 100 < chance.into()
}
