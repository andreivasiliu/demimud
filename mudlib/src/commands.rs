use string_interner::StringInterner;

use crate::{
    acting::EscapeVariables,
    acting::{Acts, InfoTarget, Players},
    colors::recolor,
    echo,
    entity::{EntityId, EntityWorld},
    socials::Socials,
    state::WorldState,
};
use crate::{
    components::Mobile,
    import::VnumTemplates,
    world::{common_direction, long_direction, opposite_direction, MobProgTrigger},
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
        &["rsay", ref message @ ..] if false => {
            // Disabled; not needed
            agent.do_mob_rsay(&message.join(" "));
        }
        &["rsay", target, ref message @ ..] if target.starts_with(">") => {
            agent.do_say_to(&target[1..], &message.join(" "));
        }
        &["rsay", ref message @ ..] => {
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
        &["open", target] => {
            agent.do_open(target);
        }
        &["close", target] => {
            agent.do_close(target);
        }
        &["i"] | &["inv"] | &["inventory"] => {
            agent.do_inventory();
        }
        &["list"] | &["wares"] => {
            agent.do_list();
        }
        &["follow", target] => {
            agent.do_follow(target);
        }
        &["unfollow"] => {
            agent.do_unfollow();
        }
        &["emote", ref message @ ..] => {
            agent.do_emote(&message.join(" "));
        }
        &[ref message @ ..] if message.len() > 0 && message[0].starts_with(",") => {
            agent.do_emote(&message.join(" ")[1..]);
        }
        &["pmote", target, ref message @ ..] => {
            agent.do_pmote(target, &message.join(" "));
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
        echo!(
            self.info(),
            "Unrecognized command: {}. Type 'help' for a list of commands.\r\n",
            cmd_word
        );
    }

    fn do_help(&mut self) {
        let help_text = include_str!("../help.txt");
        echo!(self.info(), "{}", help_text);
    }

    fn do_queue(&mut self, ticks: &str, command: String) {
        let mut myself = self.entity_world.entity_info_mut(self.entity_id);
        let ticks = match ticks.parse() {
            Ok(ticks) => ticks,
            Err(_) => {
                echo!(self.info(), "The number of ticks must be a number.\r\n");
                return;
            }
        };
        myself
            .components()
            .general
            .command_queue
            .push((ticks, command));

        echo!(self.info(), "Command queued to run in {} ticks.\r\n", ticks);
    }

    fn do_map(&mut self) {
        let map = make_map(
            &self.entity_world,
            self.entity_world.room_of(self.entity_id),
        );

        echo!(self.info(), "{}", map);
    }

    fn do_look(&mut self) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let room_id = self.entity_world.room_of(self.entity_id);
        let room = self.entity_world.entity_info(room_id);

        let mut info = self.players.info(&myself);

        // Title
        echo!(
            info,
            "\x1b[33m{}\x1b[0m\r\n",
            room.component_info().internal_title()
        );

        // Description
        let description = room.component_info().internal_description();
        echo!(info, "{}", description);
        if !description.ends_with("\r") && !description.ends_with("\n") {
            echo!(info, "\r\n");
        }

        // Exits
        let mut first_exit = true;
        for exit in room.exits() {
            if first_exit {
                first_exit = false;
                echo!(info, "`gYou see exits: ");
            } else {
                echo!(info, ", ");
            }

            echo!(info, "{}", exit.component_info().keyword());
            if let Some(door) = &exit.components().door {
                let state = if door.closed && door.locked {
                    "locked"
                } else if door.closed {
                    "closed"
                } else {
                    "open"
                };
                echo!(info, " ({})", state);
            }
        }

        if first_exit {
            echo!(info, "`gYou see no exits.`^\r\n");
        } else {
            echo!(info, ".`^\r\n");
        }

        // Objects
        for object in room.objects() {
            let container_state = match &object.components().door {
                Some(door) if door.locked => " (locked)",
                Some(door) if door.closed => " (closed)",
                Some(_) => " (opened)",
                None => "",
            };

            echo!(
                info,
                "`c{}{}`^\r\n",
                recolor("`c", object.component_info().lateral_description()),
                container_state,
            );
        }

        // Mobiles
        for mobile in room.mobiles() {
            echo!(
                info,
                "`m{}`^\r\n",
                mobile.component_info().lateral_description()
            );
        }

        // Players
        for player in room.players() {
            if player.entity_id() == self.entity_id {
                continue;
            }

            echo!(
                info,
                "`M{}`^\r\n",
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
        let mut column = 0;
        for item in target.contained_entities() {
            if item.equipped().is_none() {
                if first {
                    echo!(act.myself(), "$^$E is holding:\r\n    ");
                    first = false;
                    column = 4;
                } else {
                    echo!(act.myself(), ", ");
                    column += 2;
                }

                let short_description = item.component_info().short_description();

                if column > 4 && column + short_description.len() > 78 {
                    echo!(act.myself(), "\r\n    ");
                    column = 4;
                }

                echo!(act.myself(), "{}", short_description);
                column += short_description.len();
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
                echo!(
                    self.info(),
                    "You don't see anything named {} in the room.\r\n",
                    target_name
                );
                return;
            }
        };

        let mut info = self.players.info(&target);
        echo!(info, "You feel compelled to: {:?}\r\n", words);

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
            echo!(
                act.target(),
                "You tried to do it, but you didn't quite understand what $e meant.\r\n"
            );
            echo!(
                act.others(),
                "$^$n tries to do something, but stops with a confused look.\r\n"
            );
        }
    }

    pub fn do_say(&mut self, message: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);

        let speech_color = if myself.is_player() {
            "`M"
        } else if myself.is_mobile() {
            "`m"
        } else {
            "`c"
        };

        let message = recolor(speech_color, message);

        let (message, emote_prefix) = if message.starts_with("[") {
            let end = message.find("]").unwrap_or(message.len());
            (&message[end + 1..], Some(&message[1..end]))
        } else {
            (&message[..], None)
        };

        let (message, emote_suffix) = if message.ends_with("]") {
            let start = message
                .char_indices()
                .rev()
                .skip_while(|(_index, c)| *c != '[')
                .map(|(index, _c)| index)
                .next()
                .unwrap_or(0);
            (
                &message[..start],
                Some(&message[start + 1..message.len() - 1]),
            )
        } else {
            (&message[..], None)
        };

        let message = message.trim();

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

            let punctuation = if ends_in_punctuation || emote_suffix.is_some() {
                ""
            } else {
                "."
            };

            echo!(
                act.myself(),
                "{}You say{}{}, '{}{}{}'{}{}{}`^\r\n",
                speech_color,
                if emote_prefix.is_some() { " " } else { "" },
                emote_prefix.unwrap_or(""),
                uppercase_character,
                EscapeVariables(remaining_characters),
                punctuation,
                if emote_suffix.is_some() { ", " } else { "" },
                emote_suffix.unwrap_or(""),
                if emote_suffix.is_some() { "." } else { "" },
            );

            echo!(
                act.others(),
                "{}$^$n says{}{}, '{}{}{}'{}{}{}`^\r\n",
                speech_color,
                if emote_prefix.is_some() { " " } else { "" },
                emote_prefix.unwrap_or(""),
                uppercase_character,
                EscapeVariables(remaining_characters),
                punctuation,
                if emote_suffix.is_some() { ", " } else { "" },
                emote_suffix.unwrap_or(""),
                if emote_suffix.is_some() { "." } else { "" },
            );

            self.check_triggers_others(Action::Speech { message: &message })
        } else {
            echo!(self.info(), "You say nothing whatsoever.\r\n");
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
                echo!(
                    act.myself(),
                    "You mutter something to yourself, but nobody hears it.\r\n"
                );
                echo!(act.others(), "$^$n mutters something to himself.\r\n");
                return;
            }
            Found::Other(other) | Found::WrongOther(other) => other,
            Found::Nothing => {
                echo!(
                    self.info(),
                    "You don't see anyone named like that here.\r\n"
                );
                return;
            }
        };

        let mut act = self.players.act_with(&myself, &target);

        let speech_color = if myself.is_player() {
            "`M"
        } else if myself.is_mobile() {
            "`m"
        } else {
            "`c"
        };

        let message = recolor(speech_color, message);
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
                "{}You say to $N, '{}{}{}'`^\r\n",
                speech_color,
                uppercase_character,
                EscapeVariables(remaining_characters),
                suffix
            );

            echo!(
                act.target(),
                "{}$^$n says to you, '{}{}{}'`^\r\n",
                speech_color,
                uppercase_character,
                EscapeVariables(remaining_characters),
                suffix
            );

            echo!(
                act.others(),
                "{}$^$n says to $N, '{}{}{}'`^\r\n",
                speech_color,
                uppercase_character,
                EscapeVariables(remaining_characters),
                suffix
            );

            self.check_triggers_others(Action::Speech { message: &message })
        } else {
            echo!(self.info(), "You say nothing whatsoever.\r\n");
        }
    }

    pub(crate) fn do_recall(&mut self, location: Option<&str>) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let location = match location {
            Some(location) => location,
            None => {
                let mut info = self.info();
                echo!(info, "You can recall to:\r\n");
                let places = &[
                    "`Wrecall mekali`^ - A Large Plaza in Mekali City",
                    "`Wrecall gnomehill`^ - A Large Plaza on Gnome Hill",
                    "`Wrecall dzagari`^ - The Blasted Square in Dzagari",
                    "`Wrecall mudschool`^ - The Welcome room in MudSchool",
                ];
                for place in places {
                    echo!(info, " {}\r\n", place);
                }
                return;
            }
        };

        if let Some(room_id) = self.entity_world.landmark(location) {
            let mut act = self.players.act_alone(&myself);
            echo!(
                act.myself(),
                "You close your eyes in prayer, and feel your surroundings shift around you.\r\n",
            );
            echo!(
                act.others(),
                "$n close $s eyes in prayer, and fades out into thin air.\r\n",
            );

            self.entity_world.move_entity(self.entity_id, room_id);
            self.do_look();

            // A temporary substitute for logging in to make it easier to test
            self.check_triggers_others(Action::Login);
        } else {
            echo!(
                self.info(),
                "Unknown location; type `Wrecall`^ to see a list.\r\n"
            );
        }
    }

    pub fn do_move(&mut self, direction: &str) -> bool {
        let direction = long_direction(direction);

        let myself = self.entity_world.entity_info(self.entity_id);
        let target = myself.find_entity(direction, |entity| entity.is_exit());

        let exit = match target {
            Found::Myself => {
                // Not hard to implement, but not worth the effort.
                echo!(
                    self.info(),
                    "That's you! And.. wait, you're a valid exit to go into? But still, no.\r\n"
                );
                return true;
            }
            Found::Other(exit) => exit,
            Found::WrongSelf => {
                echo!(
                    self.info(),
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
                    echo!(self.info(), "The way to the {} is blocked.\r\n", direction);
                    return true;
                } else {
                    return false;
                }
            }
        };

        let mut act = self.players.act_alone(&myself);

        let exit_keyword = exit.component_info().keyword();

        let from_room_id = myself.room().entity_id();
        let to_room_id = match exit.leads_to() {
            Some(room_id) => room_id,
            None => {
                echo!(act.myself(), "That exit leads into the void!\r\n");
                echo!(
                    act.others(),
                    "$^$n leaves {} but returns with a confused look.\r\n",
                    exit_keyword
                );
                return true;
            }
        };

        let exit_id = exit.entity_id();

        echo!(act.myself(), "You walk {}.\r\n", exit_keyword);
        echo!(act.others(), "$^$n leaves {}.\r\n", exit_keyword);
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

        // Allow followers to admire new surroundings.
        self.check_followers(from_room_id, direction, to_room_id);

        // Others might admire you.
        self.check_triggers_others(Action::Greet);

        true
    }

    pub fn do_exits(&mut self) {
        let room_id = self.entity_world.room_of(self.entity_id);
        let room = self.entity_world.entity_info(room_id);

        let myself = self.entity_world.entity_info(self.entity_id);
        let mut info = self.players.info(&myself);

        echo!(info, "You see the following exits:\r\n");
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
                info,
                "  `W{}`^: {} leading to `y{}`^.\r\n",
                exit.component_info().keyword(),
                exit.component_info().short_description(),
                other_room
            )
        }
    }

    pub fn do_emote(&mut self, message: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself).store_acts();

        if message.contains('*') {
            // Allow players to suppress the space after the name (e.g. "*'s eyes shine")
            // or to put the name in a different place.
            echo!(
                act.myself(),
                "You emote: $^{}\r\n",
                message.replace("*", "$n")
            );
            echo!(act.others(), "$^{}\r\n", message.replace("*", "$n"));
        } else {
            echo!(act.myself(), "You emote: $^$n {}\r\n", message);
            echo!(act.others(), "$^$n {}\r\n", message);
        }

        let acts = act.into_acts();
        self.check_act_triggers(acts);
    }

    pub fn do_pmote(&mut self, target: &str, message: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let target = myself.find_entity(target, |e| {
            !e.is_extra_description() && e.entity_id() != myself.entity_id()
        });

        let target = match target {
            Found::Myself | Found::WrongSelf => {
                echo!(self.info(), "You can't pmote yourself, use an untargetted '`Wemote`^' instead.\r\n");
                return;
            }
            Found::Nothing | Found::WrongOther(_) => {
                echo!(self.info(), "You don't see anything like that here.\r\n");
                return;
            }
            Found::Other(other) => other,
        };

        let mut act = self.players.act_with(&myself, &target).store_acts();
        let target_name = target.component_info().short_description();
        let you_or_target = format!("[you/{}]", target_name);

        if message.contains('*') {
            // Allow players to suppress the space after the name (e.g. "*'s eyes shine")
            // or to put the name in a different place.
            let message = message.replace("*", "$n");
            echo!(
                act.myself(),
                "You emote: $^{}\r\n",
                message.replace("@", &you_or_target)
            );

            echo!(act.target(), "$^{}\r\n", message.replace("@", "you"));
            echo!(act.others(), "$^{}\r\n", message.replace("@", target_name));
        } else {
            let you_or_target = format!("[you/{}]", target_name);
            echo!(act.myself(), "You emote: $^$n {}\r\n", message.replace("@", &you_or_target));
            echo!(act.target(), "$^$n {}\r\n", message.replace("@", "you"));
            echo!(act.others(), "$^$n {}\r\n", message.replace("@", target_name));
        }

        let acts = act.into_acts();
        self.check_act_triggers(acts);
    }

    pub fn do_socials(&mut self, social: Option<&str>) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut info = self.players.info(&myself);

        if let Some(social) = social {
            if let Some(social) = self.socials.get(social) {
                echo!(
                    info,
                    "The {} emote shows the following messages:\r\n",
                    social.name
                );
                echo!(info, "Untargetted:\r\n");
                echo!(info, "  \"{}\"\r\n", &social.untargetted_self);
                echo!(info, "  \"{}\"\r\n", &social.untargetted_others);

                echo!(info, "Targetted:\r\n");
                echo!(info, "  \"{}\"\r\n", &social.targetted_self);
                echo!(info, "  \"{}\"\r\n", &social.targetted_target);
                echo!(info, "  \"{}\"\r\n", &social.targetted_others);

                echo!(info, "Self-targetted:\r\n");
                echo!(info, "  \"{}\"\r\n", &social.reflected_self);
                echo!(info, "  \"{}\"\r\n", &social.reflected_others);
            } else {
                echo!(info, "There is no social with that name.\r\n");
            }
        } else {
            echo!(info, "The following emotes are available:\r\n");

            let mut column = 0;
            let mut first = true;

            for social in self.socials.list() {
                if first {
                    first = false;
                } else {
                    echo!(info, ", ");
                    column += 2;

                    if column > 70 {
                        echo!(info, "\r\n");
                        column = 0;
                    }
                }

                echo!(info, "`W{}`^", social);
                column += social.len();
            }

            echo!(info, ".\r\n");
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
                    echo!(
                        self.info(),
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
                echo!(self.info(), "Get what?\r\n");
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
                echo!(
                    self.info(),
                    "You try to get a hold of yourself. You think you succeeded.\r\n"
                );
                return;
            }
            Found::Other(other) => {
                let mut act = self.players.act_with(&myself, &other).store_acts();
                echo!(act.myself(), "You pick up $N.\r\n");
                echo!(
                    act.target(),
                    "$^$n picks you up. You're now in $s inventory!\r\n"
                );
                echo!(act.others(), "$^$n gets $N.\r\n");
                let acts1 = act.into_acts();

                let myself_id = myself.entity_id();
                let other_id = other.entity_id();

                self.entity_world.move_entity(other_id, myself_id);

                let other = self.entity_world.entity_info(other_id);
                let mut act = self.players.act_alone(&other).store_acts();
                echo!(
                    act.others(),
                    "$^$n is tossed into here, and lands with a thud.\r\n"
                );
                let acts2 = act.into_acts();

                // Check triggers only have everyone saw the message, so that
                // the events are seen in order.
                self.check_act_triggers(acts1);
                self.check_act_triggers(acts2);
            }
            Found::Nothing | Found::WrongOther(_) => {
                echo!(
                    self.info(),
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
                echo!(
                    self.info(),
                    "You aren't holding anything named like that.\r\n"
                );
                return;
            }
        }
    }

    pub fn do_open(&mut self, target: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let target = myself.find_entity(target, |entity| {
            // Prefer closed doors
            if let Some(ref door) = entity.components().door {
                door.closed
            } else {
                false
            }
        });

        let target = match target {
            Found::Myself | Found::WrongSelf => {
                // Not necessarily impossible, but doesn't seem useful.
                echo!(
                    self.info(),
                    "You can't seem to open yourself, you'll need some assistance.\r\n"
                );
                return;
            }
            Found::Nothing => {
                echo!(
                    self.info(),
                    "You don't see anything here by that name to open.\r\n"
                );
                return;
            }
            Found::Other(other) | Found::WrongOther(other) => other,
        };

        let target_id = target.entity_id();
        let mut target = self.entity_world.entity_info_mut(target_id);

        let door = match &mut target.components().door {
            Some(door) if door.locked => Err("appears to be locked"),
            Some(door) if !door.closed => Err("is already open"),
            Some(door) => Ok(door),
            None => Err("is not something you can open"),
        };

        let door = match door {
            Ok(door) => door,
            Err(err) => {
                let myself = self.entity_world.entity_info(self.entity_id);
                let target = self.entity_world.entity_info(target_id);
                let mut act = self.players.act_with(&myself, &target);
                echo!(act.myself(), "But $N {}!\r\n", err);
                return;
            }
        };

        door.closed = false;

        let myself = self.entity_world.entity_info(self.entity_id);
        let target = self.entity_world.entity_info(target_id);
        let mut act = self.players.act_with(&myself, &target).store_acts();
        echo!(act.myself(), "You open $N.\r\n");
        echo!(act.target(), "$^$n opens you.\r\n");
        echo!(act.others(), "$^$n opens $N.\r\n");

        let acts = act.into_acts();
        self.check_act_triggers(acts);
    }

    pub fn do_close(&mut self, _target: &str) {}

    pub fn do_inventory(&mut self) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let mut info = self.players.info(&myself);
        echo!(info, "You are holding:\r\n    ");
        let mut first = true;
        let mut column = 4;
        for item in myself.contained_entities() {
            if first {
                first = false;
            } else {
                echo!(info, ", ");
                column += 2;
            }

            if column > 72 {
                echo!(info, "\r\n    ");
                column = 4;
            }

            let short_description = item.component_info().short_description();
            echo!(info, "{}", short_description);
            column += short_description.len();
        }
        echo!(info, "\r\n");
    }

    pub fn do_list(&mut self) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let room = myself.room();

        let shopkeeper = room
            .contained_entities()
            .filter_map(|entity| match &entity.components().mobile {
                Some(Mobile {
                    shopkeeper: Some(shopkeeper),
                    ..
                }) => Some((entity, shopkeeper)),
                Some(_) | None => None,
            })
            .next();

        if let Some((entity, shop_info)) = shopkeeper {
            let mut act = self.players.act_with(&myself, &entity);
            echo!(
                act.target(),
                "$^$n asks you about your wares, and you show $m what you have.\r\n"
            );
            echo!(act.others(), "$^$n asks $N about $S wares.\r\n");

            echo!(act.myself(), "$^$N shows you $S wares:\r\n");

            let mut info = self.players.info(&myself);
            for item in entity.objects() {
                if let Some(object_info) = &item.components().object {
                    let price = object_info.cost * shop_info.profit_buy as i32 / 100;
                    echo!(
                        info,
                        "  {}: `W{}`^ silver coins\r\n",
                        item.component_info().short_description(),
                        price
                    );
                } else {
                    echo!(
                        info,
                        "  {}: priceless\r\n",
                        item.component_info().short_description()
                    );
                }
            }
        } else {
            echo!(self.info(), "You don't see any shopkeepers here.\r\n");
        }
    }

    pub fn do_follow(&mut self, target: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let target = myself.find_entity(target, |entity| {
            !entity.is_extra_description() && entity.entity_id() != self.entity_id
        });

        let target = match target {
            Found::Myself | Found::WrongSelf => {
                self.do_unfollow();
                return;
            }
            Found::Nothing | Found::WrongOther(_) => {
                echo!(
                    self.info(),
                    "You don't see anyone like that to follow here.\r\n"
                );
                return;
            }
            Found::Other(other) => other,
        };

        let mut act = self.players.act_with(&myself, &target);
        echo!(act.myself(), "You start following $N.\r\n");
        echo!(act.target(), "$^$n starts following you.\r\n");
        echo!(act.others(), "$^$n starts following $N.\r\n");

        let main_keyword = target.main_keyword().to_string();
        let mut myself = self.entity_world.entity_info_mut(self.entity_id);
        myself.components().general.following = Some(main_keyword);
    }

    pub fn do_unfollow(&mut self) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);
        echo!(act.myself(), "You stop following anyone.\r\n");
        echo!(act.others(), "$^$n stops following anyone.\r\n");

        let mut myself = self.entity_world.entity_info_mut(self.entity_id);
        myself.components().general.following = None;
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
                self.do_mob_call(p_vnum, target);
            }
            &["remember", target] => {
                self.do_mob_remember(target);
            }
            &["rsay", ref message @ ..] => {
                self.do_mob_rsay(&message.join(" "));
            }
            &["force", target, ref command @ ..] => {
                // No difference from normal command
                self.do_force(target, command);
            }
            &["mpfollow", target] => {
                // No difference from normal command
                self.do_follow(target);
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
                echo!(
                    self.info(),
                    "Transfer room target '{}' is not a valid vnum.\r\n",
                    to_room
                );
                return;
            }
        };

        let room = self
            .vnum_templates
            .vnum_to_entity
            .get(to_room_vnum)
            .and_then(|permanent_id| *permanent_id)
            .and_then(|permanent_id| self.entity_world.old_entity(&permanent_id));

        let room_id = match room {
            Some(entity) => entity.entity_id(),
            None => {
                echo!(
                    self.info(),
                    "Transfer room target '{}' was destroyed.\r\n",
                    to_room
                );
                return;
            }
        };

        let target = myself.find_entity(target_name, |_| true);

        let target_id = match target {
            Found::Myself | Found::WrongSelf => myself.entity_id(),
            Found::Other(other) | Found::WrongOther(other) => other.entity_id(),
            Found::Nothing => {
                echo!(
                    self.info(),
                    "I don't see anyone here by that name to transfer.\r\n"
                );
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
        echo!(
            act.others(),
            "$^$n appears into the room out of thin air.\r\n"
        );
    }

    pub fn do_mob_dequeue_all(&mut self) {
        let mut myself = self.entity_world.entity_info_mut(self.entity_id);
        let queue = &mut myself.components().general.command_queue;

        let commands = queue.len();
        queue.clear();

        echo!(
            self.info(),
            "Cleared {} commands from the queue.\r\n",
            commands
        );
    }

    pub fn do_mob_at(&mut self, at_room: &str, commands: &[&str]) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let at_room_vnum: usize = match at_room.parse() {
            Ok(vnum) => vnum,
            Err(_) => {
                echo!(self.info(), "Room '{}' is not a valid vnum.\r\n", at_room);
                return;
            }
        };

        let room = self
            .vnum_templates
            .vnum_to_entity
            .get(at_room_vnum)
            .and_then(|permanent_id| *permanent_id)
            .and_then(|permanent_id| self.entity_world.old_entity(&permanent_id));

        let room_id = match room {
            Some(entity) => entity.entity_id(),
            None => {
                echo!(self.info(), "Room '{}' was destroyed.\r\n", at_room);
                return;
            }
        };

        let original_room_id = myself.room().entity_id();

        self.entity_world.move_entity(self.entity_id, room_id);
        process_agent_command(self, commands);
        self.entity_world
            .move_entity(self.entity_id, original_room_id);
    }

    pub fn do_mob_mload(&mut self, m_vnum: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let m_vnum: usize = match m_vnum.parse() {
            Ok(vnum) => vnum,
            Err(_) => {
                echo!(self.info(), "Vnum '{}' is not a valid number.\r\n", m_vnum);
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
                echo!(
                    self.info(),
                    "Mobile template with vnum '{}' does not exist.\r\n",
                    m_vnum
                );
                return;
            }
        };

        let room_id = myself.room().entity_id();

        let mobile_id = self
            .entity_world
            .insert_entity(room_id, mobile_components.clone());
        for mobprog in mobprogs {
            self.entity_world.insert_entity(mobile_id, mobprog.clone());
        }

        let myself = self.entity_world.entity_info(self.entity_id);
        let mobile = self.entity_world.entity_info(mobile_id);
        let mut act = self.players.act_with(&mobile, &myself);
        echo!(
            act.target(),
            "Spawned $N from mobile template with vnum '{}' .\r\n",
            m_vnum
        );
        echo!(
            act.myself(),
            "You have been spawned by $n into existence. Welcome!\r\n"
        );
        echo!(
            act.others(),
            "$^$n creates $N from thin air and drops $M into the room.\r\n"
        );
    }

    pub fn do_mob_call(&mut self, p_vnum: &str, target: &str) {
        let p_vnum: usize = match p_vnum.parse() {
            Ok(vnum) => vnum,
            Err(_) => {
                echo!(self.info(), "Vnum '{}' is not a valid number.\r\n", p_vnum);
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
                echo!(
                    self.info(),
                    "MobProg with vnum '{}' does not exist.\r\n",
                    p_vnum
                );
                return;
            }
        };

        self.run_mobprog(code, target.to_string());
    }

    pub fn do_mob_remember(&mut self, target: &str) {
        let mut myself = self.entity_world.entity_info_mut(self.entity_id);

        let remembered = if let Some(mobile) = &mut myself.components().mobile {
            mobile.remember = Some(target.to_string());

            true
        } else {
            false
        };

        if remembered {
            echo!(self.info(), "Target remembered.\r\n");
        } else {
            echo!(
                self.info(),
                "But you are not a mobile! You can't remember things.\r\n"
            );
        }
    }

    pub fn do_mob_rsay(&mut self, message: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let target = if let Some(mobile) = &myself.components().mobile {
            if let Some(target) = &mobile.remember {
                Ok(target)
            } else {
                Err("But you don't have anyone remembered!")
            }
        } else {
            Err("But you are not a mobile! You can't remember things.")
        };

        let target = match target {
            Ok(target) => target.clone(),
            Err(err) => {
                echo!(self.info(), "{}\r\n", err);
                return;
            }
        };

        self.do_say_to(&target, message);
    }
}

pub(crate) enum Action<'a> {
    /// A word or phrase is said
    Speech { message: &'a str },

    /// Someone enters your room
    Greet,

    /// You entered a new room
    Entry,

    /// You spawned or recalled here
    Login,
}

impl<'e, 'p> EntityAgent<'e, 'p> {
    fn info(&mut self) -> InfoTarget<'_> {
        let myself = self.entity_world.entity_info(self.entity_id);
        self.players.info(&myself)
    }

    fn check_followers(&mut self, from_room_id: EntityId, direction: &str, to_room_id: EntityId) {
        let mut followers = Vec::new();
        let myself = self.entity_world.entity_info(self.entity_id);
        let room = self.entity_world.entity_info(from_room_id);

        for follower in room.contained_entities() {
            let main_keyword = myself.main_keyword();
            if let Some(following) = &follower.components().general.following {
                if following == main_keyword {
                    followers.push(follower.entity_id());

                    let mut act = self.players.act_with(&follower, &myself);
                    echo!(act.myself(), "You follow $N to the {}.\r\n", direction);
                    echo!(act.others(), "$^$n follows $N to the {}.\r\n", direction);
                }
            }
        }

        for follower_id in followers {
            let myself = self.entity_world.entity_info(self.entity_id);
            let follower = self.entity_world.entity_info(follower_id);
            let mut act = self.players.act_with(&follower, &myself);
            echo!(
                act.target(),
                "$^$n follows you in from the {}.\r\n",
                opposite_direction(direction)
            );
            echo!(
                act.others(),
                "$^$n follows $N in from the {}.\r\n",
                opposite_direction(direction)
            );

            self.entity_world.move_entity(follower_id, to_room_id);
            let mut agent = self.switch_agent(follower_id);
            agent.do_look();
            agent.check_triggers_self(Action::Entry);
        }
    }

    fn check_triggers_self(&mut self, action: Action<'_>) {
        let mut triggered = Vec::new();
        let myself = self.entity_world.entity_info(self.entity_id);
        for item in myself.contained_entities() {
            if let Some(mobprog) = &item.components().mobprog {
                match (&action, &mobprog.trigger) {
                    (Action::Entry, MobProgTrigger::Entry { chance }) => {
                        if random_percent(*chance) {
                            triggered.push(mobprog.code.clone());
                        }
                    }
                    _ => (),
                }
            }
        }

        if triggered.is_empty() {
            return;
        }

        let self_keyword = myself.main_keyword().to_string();

        for code in triggered {
            // TODO: Maybe running with this target isn't how it's supposed
            // to work?
            self.run_mobprog(code, self_keyword.clone());
        }
    }

    fn check_triggers_others(&mut self, action: Action<'_>) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut triggered = Vec::new();

        if !myself.is_player() {
            return;
        }

        for entity in myself.room().contained_entities() {
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
                        (Action::Entry, MobProgTrigger::Entry { chance }) => {
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

    pub fn check_act_triggers(&mut self, acts: Acts) {
        // Note: `&mut self` might no longer be acts.myself_entity_id.
        let myself = self.entity_world.entity_info(acts.myself_entity_id);
        let mut triggered = Vec::new();

        for entity in myself.room().contained_entities() {
            for mobprog in entity.contained_entities() {
                let mobprog = match &mobprog.components().mobprog {
                    Some(mobprog) => mobprog,
                    None => continue,
                };

                let lines = if entity.entity_id() == acts.myself_entity_id {
                    acts.myself.lines()
                } else if Some(entity.entity_id()) == acts.target_entity_id {
                    acts.target.lines()
                } else {
                    acts.others.lines()
                };

                for line in lines {
                    if let MobProgTrigger::Act { pattern } = &mobprog.trigger {
                        println!("Checking '{}' against '{}'", pattern, line);
                        if line.contains(pattern) {
                            println!("Matched!");
                            triggered.push((entity.entity_id(), mobprog.code.clone()));
                        }
                    }
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
        let mut accept_commands = true;

        for command in code.lines() {
            let command = command.replace("$n", &target);

            let myself = self.entity_world.entity_info(self.entity_id);
            let remembered = myself
                .components()
                .mobile
                .as_ref()
                .and_then(|mobile| mobile.remember.as_deref())
                .unwrap_or("nobody");

            let command = command.replace("$q", remembered);

            println!("Processing: {}", command);

            if command.trim_start().starts_with("**") {
                continue;
            }

            let words: Vec<_> = command.split_whitespace().collect();

            let remembered = myself
                .components()
                .mobile
                .as_ref()
                .and_then(|mobile| mobile.remember.as_deref())
                .unwrap_or("nobody");

            match &words[..] {
                &["if", ref condition @ ..] => {
                    accept_commands = match condition {
                        &["room", target, "==", vnum] => {
                            assert!(["$i", "$I"].contains(&target), "Don't know how to handle other targets");

                            let vnum: usize = vnum.parse().expect("Invalid vnum in mobprog");

                            let myself = self.entity_world.entity_info(self.entity_id);
                            let room = myself.room();

                            vnum == room.components().general.vnum.0
                        }
                        &["istarget", target] => {
                            target == remembered
                        }
                        &["!istarget", target] => {
                            target != remembered
                        }
                        _ => false,
                    };
                }
                &["else"] => accept_commands = !accept_commands,
                &["endif"] => accept_commands = true,
                &["end"] if accept_commands => break,
                command if accept_commands => {
                    process_agent_command(self, command);
                }
                _ => println!("Ignored."),
            };
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
