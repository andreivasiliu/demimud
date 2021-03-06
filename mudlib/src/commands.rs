use crate::{
    acting::EscapeVariables,
    agent::EntityAgent,
    colors::recolor,
    components::{Door, Mobile, Object},
    echo,
    entity::Found,
    files::fix_newlines,
    find_entities::EntityIterator,
    mapper::make_map,
    mobprogs::Action,
    state::WorldState,
    world::{common_direction, long_direction, opposite_direction, Shop},
};

pub(crate) fn process_agent_command(agent: &mut EntityAgent, words: &[&str]) -> bool {
    match *words {
        ["panic"] => {
            panic!("Oh no! I panicked!");
        }
        ["help"] => {
            agent.do_help(None);
        }
        ["help", help_file] => {
            agent.do_help(Some(help_file));
        }
        ["die"] => {
            agent.do_die();
        }
        ["areas"] => {
            agent.do_areas();
        }
        ["buy", item] => {
            agent.do_buy(item);
        }
        ["sell", item] => {
            agent.do_sell(item);
        }
        ["eat", item] => {
            agent.do_eat(item, false);
        }
        ["eat", item, "forcefully"] => {
            agent.do_eat(item, true);
        }
        ["mq", ticks, ref command @ ..] => {
            agent.do_queue(ticks, command.join(" "));
        }
        [mq, ref command @ ..] if mq.starts_with("mq") && mq[2..].parse::<u32>().is_ok() => {
            agent.do_queue(&mq[2..], command.join(" "));
        }
        ["mob", ref command @ ..] => {
            agent.do_mob(command);
        }
        ["map"] => {
            agent.do_map();
        }
        ["look"] | ["l"] | ["examine"] => {
            agent.do_look();
        }
        [look, target] | [look, "at", target] if ["look", "l", "examine"].contains(&look) => {
            agent.do_look_at(target);
        }
        ["look", ..] | ["l", ..] | ["examine", ..] => {
            echo!(agent.info(), "Syntax: '`Wlook <word>`^'\r\n");
        }
        ["force", target, ref victim_words @ ..] => {
            agent.do_force(target, victim_words);
        }
        ["say", target, ref message @ ..] if target.starts_with('>') => {
            agent.do_say_to(&target[1..], &message.join(" "));
        }
        ["sayto", target, ref message @ ..] => {
            agent.do_say_to(target, &message.join(" "));
        }
        ["say", ref message @ ..] => {
            agent.do_say(&message.join(" "));
        }
        [target, ref message @ ..] if target.starts_with("'>") => {
            agent.do_say_to(&target[2..], &message.join(" "));
        }
        [ref message @ ..] if !message.is_empty() && message[0].starts_with('\'') => {
            agent.do_say(&message.join(" ")[1..]);
        }
        ["rsay", ref message @ ..] if false => {
            // Disabled; not needed
            agent.do_mob_rsay(&message.join(" "));
        }
        ["rsay", target, ref message @ ..] if target.starts_with('>') => {
            agent.do_say_to(&target[1..], &message.join(" "));
        }
        ["rsay", ref message @ ..] => {
            agent.do_say(&message.join(" "));
        }
        ["recall"] => {
            agent.do_recall(None);
        }
        ["recall", location] => {
            agent.do_recall(Some(location));
        }
        ["exits"] => {
            agent.do_exits();
        }
        ["get", "all"] => {
            agent.do_get_all(false);
        }
        ["drop", "all"] => {
            agent.do_drop_all(false);
        }
        ["get"] => {
            agent.do_get(None, false);
        }
        ["get", item] => {
            agent.do_get(Some(item), false);
        }
        ["get", item, "forcefully"] => {
            agent.do_get(Some(item), true);
        }
        ["get", item, container] => {
            agent.do_get_from(item, container, false);
        }
        ["get", item, "from", container] => {
            agent.do_get_from(item, container, false);
        }
        ["get", item, container, "forcefully"] => {
            agent.do_get_from(item, container, true);
        }
        ["get", item, "from", container, "forcefully"] => {
            agent.do_get_from(item, container, true);
        }
        ["put", item, container] => {
            agent.do_put_into(item, container, false);
        }
        ["put", item, "into", container] => {
            agent.do_put_into(item, container, false);
        }
        ["put", item, container, "forcefully"] => {
            agent.do_put_into(item, container, true);
        }
        ["put", item, "into", container, "forcefully"] => {
            agent.do_put_into(item, container, true);
        }
        ["drop"] => {
            agent.do_drop(None, false);
        }
        ["drop", item] => {
            agent.do_drop(Some(item), false);
        }
        ["drop", item, "forcefully"] => {
            agent.do_drop(Some(item), true);
        }
        ["give", item, target] | ["give", item, "to", target] => {
            agent.do_give(item, target, false);
        }
        ["give", item, target, "forcefully"] | ["give", item, "to", target, "forcefully"] => {
            agent.do_give(item, target, true);
        }
        ["give", ..] => {
            echo!(agent.info(), "Syntax: `Wgive <item> [to] <target>`^\r\n");
        }
        ["open", target] => {
            agent.do_open(target);
        }
        ["close", target] => {
            agent.do_close(target);
        }
        ["unlock", target] => {
            agent.do_unlock(target);
        }
        ["lock", target] => {
            agent.do_lock(target);
        }
        ["i"] | ["inv"] | ["inventory"] => {
            agent.do_inventory();
        }
        ["list"] | ["wares"] => {
            agent.do_list();
        }
        ["follow", target] => {
            agent.do_follow(target);
        }
        ["unfollow"] => {
            agent.do_unfollow();
        }
        ["emote", ref message @ ..] => {
            agent.do_emote(&message.join(" "));
        }
        [ref message @ ..] if !message.is_empty() && message[0].starts_with(',') => {
            agent.do_emote(&message.join(" ")[1..]);
        }
        ["pmote", target, ref message @ ..] => {
            agent.do_pmote(target, &message.join(" "));
        }
        ["social"] | ["socials"] | ["emotes"] => {
            agent.do_socials(None);
        }
        ["social", social] | ["socials", social] | ["emotes", social] => {
            agent.do_socials(Some(social));
        }
        [direction] if agent.do_move(direction) => (),
        [social] if agent.do_social(social, None) => (),
        [social, target] if agent.do_social(social, Some(target)) => (),
        [cmd_word, ..] => {
            agent.do_unknown(cmd_word);
            return false;
        }
        [] => (),
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
                    .push_str("But you don't seem to have a body.\r\n");
            }
            return;
        }
    };

    let mut agent = EntityAgent {
        entity_world: &mut world_state.entity_world,
        socials: &world_state.socials,
        vnum_templates: &world_state.vnum_templates,
        areas: &world_state.areas,
        players: &mut world_state.players,
        entity_id: player_id,
    };

    process_agent_command(&mut agent, words);
}

impl<'e, 'p> EntityAgent<'e, 'p> {
    pub fn do_unknown(&mut self, cmd_word: &str) {
        echo!(
            self.info(),
            "Unrecognized command: {}. Type 'help' for a list of commands.\r\n",
            cmd_word
        );
    }

    pub fn do_help(&mut self, help_file: Option<&str>) {
        let help_text = match help_file {
            Some("commands") => include_str!("../help_commands.txt"),
            Some("emote") => include_str!("../help_emote.txt"),
            Some("cli") => include_str!("../help_cli.txt"),
            Some("demimud") => include_str!("../help_demimud.txt"),
            Some("credits") => include_str!("../help_credits.txt"),
            None => include_str!("../help.txt"),
            _ => "Unknown help file. See '`Whelp`^' without an argument.\r\n",
        };
        echo!(self.info(), "{}", fix_newlines(help_text));
    }

    pub fn do_die(&mut self) {
        let myself = self.entity_world.entity_info(self.entity_id);

        if myself.is_mobile() || myself.is_player() {
            let mut act = self.players.act_alone(&myself);
            echo!(act.myself(), "You are `RDEAD`^.\r\n");
            echo!(act.others(), "$n is `RDEAD`^.\r\n");

            let holder = myself.room();
            let mut act = self.players.act_with(&holder, &myself);
            echo!(act.myself(), "$^$N is `RDEAD`^.\r\n");
            echo!(act.others(), "$^$N is `RDEAD`^.\r\n");
        }

        let limbo = self
            .entity_world
            .landmark("limbo")
            .expect("Limbo should always exist");
        self.entity_world.move_entity(self.entity_id, limbo);
    }

    pub fn do_areas(&mut self) {
        echo!(self.info(), "Areas:\r\n");

        for area in self.areas {
            // Expensive, but let's honor them properly.
            let credits = area
                .credits
                .split_whitespace()
                .map(|builder| format!("`M{}`^", builder))
                .collect::<Vec<_>>()
                .join(", ");

            echo!(
                self.info(),
                "`C{:>32}`^ - `g{:>5}`^..`g{:>5}`^ - {}\r\n",
                area.name,
                area.vnums.0 .0,
                area.vnums.1 .0,
                credits,
            );
        }
    }

    pub fn do_buy(&mut self, item_name: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let found = myself
            .room()
            .contained_entities()
            .with_component::<Shop>()
            .filter_or(
                |e| *e != myself,
                "The only shopkeeper here is yourself.\r\n",
            )
            .find_one_with_component_or("You don't see any shopkeepers here.");

        let (entity, shop_info) = match found {
            Ok(found) => found,
            Err(error) => return self.echo_error(error),
        };

        let shopkeeper_id = entity.entity_id();

        let found = entity
            .objects()
            .filter_by_keyword(item_name)
            .with_component::<Object>()
            .find_one_with_component_or("You don't see anything named like that to buy.");

        let (item, object) = match found {
            Ok(item) => item,
            Err(error) => return self.echo_error(error),
        };

        let item_id = item.entity_id();
        let cost = (object.cost * shop_info.profit_buy as i32 / 100) as usize;

        if self.remove_silver(cost, self.entity_id) {
            // Clone it so that the shopkeeper can keep selling it
            let item = self.entity_world.entity_info(item_id);
            let components = item.components().clone();
            self.entity_world.insert_entity(self.entity_id, components);

            let myself = self.entity_world.entity_info(self.entity_id);
            let shopkeeper = self.entity_world.entity_info(shopkeeper_id);
            let item = self.entity_world.entity_info(item_id);
            let mut act = self.players.act_with(&myself, &shopkeeper);
            echo!(
                act.myself(),
                "You buy {} from $N for {} silver.\r\n",
                item,
                cost
            );
            echo!(
                act.target(),
                "$^$n buys {} from you for {} silver.\r\n",
                item,
                cost
            );
            echo!(act.others(), "$^$n buys {} from $N.\r\n", item);
        } else {
            echo!(
                self.info(),
                "You don't have the {} silver to pay for it!\r\n",
                cost
            );
        }
    }

    pub fn do_sell(&mut self, item_name: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let found = myself
            .room()
            .contained_entities()
            .with_component::<Shop>()
            .filter_or(
                |e| *e != myself,
                "The only shopkeeper here is yourself.\r\n",
            )
            .find_one_with_component_or("You don't see any shopkeepers here.");

        let (entity, shop_info) = match found {
            Ok(found) => found,
            Err(error) => return self.echo_error(error),
        };

        let shopkeeper_id = entity.entity_id();

        let found = myself
            .objects()
            .filter_by_keyword(item_name)
            .with_component_or::<Object>("$^$N is not an object you can sell.")
            .find_one_with_component_or("You don't own anything named like that to sell.");

        let (item, object) = match found {
            Ok(found) => found,
            Err(error) => return self.echo_error(error),
        };

        // FIXME: fix integer types
        let cost = (object.cost * shop_info.profit_sell as i32 / 100) as usize;
        let item_id = item.entity_id();

        self.add_silver(cost, self.entity_id);
        let mut item_agent = self.switch_agent(item_id);
        item_agent.do_die();

        let myself = self.entity_world.entity_info(self.entity_id);
        let shopkeeper = self.entity_world.entity_info(shopkeeper_id);
        let item = self.entity_world.entity_info(item_id);
        let mut act = self.players.act_with(&myself, &shopkeeper);
        echo!(
            act.myself(),
            "You sell {} to $N for {} silver.\r\n",
            item,
            cost
        );
        echo!(
            act.target(),
            "$^$n sells {} to you for {} silver.\r\n",
            item,
            cost
        );
        echo!(act.others(), "$^$n sells {} to $N.\r\n", item);
    }

    pub fn do_eat(&mut self, item_name: &str, forcefully: bool) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let found = myself
            .contained_entities()
            .filter_by_keyword(item_name)
            .filter_or(
                |food| {
                    forcefully
                        || food.components().object.as_ref().map(|object| object.food) == Some(true)
                },
                "$^$N does not appear to be edible.",
            )
            .find_one_or("You aren't holding any objects named like that.");

        let food = match found {
            Ok(found) => found,
            Err(error) => return self.echo_error(error),
        };

        let mut act = self.players.act_with(&myself, &food);
        echo!(act.myself(), "You eat $N with gusto.\r\n");
        echo!(act.target(), "$^$n devours you whole with gusto.\r\n");
        echo!(act.others(), "$^$n eats $N with gusto.\r\n");

        let mut act = self.players.act_with(&food, &myself);
        echo!(act.others(), "$^$n is devoured whole by $N.\r\n");

        if food.is_mobile() || food.is_player() {
            let mut act = self.players.act_with(&myself, &food);
            echo!(
                act.myself(),
                "$^$N struggles for a bit, but you eventually devour $M whole.\r\n"
            );
            echo!(
                act.target(),
                "You struggle for a bit, but are eventually devoured whole.\r\n"
            );
            echo!(
                act.others(),
                "$^$N struggles for a bit, but is eventually devoured whole.\r\n"
            );
        }

        let food_id = food.entity_id();

        let mut agent = self.switch_agent(food_id);
        agent.do_die();
    }

    pub fn do_queue(&mut self, ticks: &str, command: String) {
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

    pub fn do_map(&mut self) {
        let map = make_map(
            &self.entity_world,
            self.entity_world.room_of(self.entity_id),
        );

        echo!(self.info(), "{}", map);
    }

    pub fn do_look(&mut self) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let room_id = self.entity_world.room_of(self.entity_id);
        let room = self.entity_world.entity_info(room_id);

        let mut info = self.players.info(&myself);

        // Title
        echo!(info, "`y{}`^\r\n", room.component_info().internal_title());

        // Description
        let description = room.component_info().internal_description();
        echo!(info, "{}", description);
        if !description.ends_with('\r') && !description.ends_with('\n') {
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

            echo!(info, "{}", exit.main_keyword());
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

        let found = myself
            .visible_entities(target)
            .filter_or(|e| *e != myself, "It's you! But you're sadly just a player and players don't have descriptions yet.\r\n")
            .find_one_or("You don't see anything named like that here.\r\n");

        let target = match found {
            Ok(target) => target,
            Err(error) => return self.echo_error(error),
        };

        // Description
        let description = target.component_info().external_description();
        let newline = if description.ends_with('\r') || description.ends_with('\n') {
            ""
        } else {
            "\r\n"
        };

        let mut act = self.players.act_alone(&myself);
        echo!(act.myself(), "{}{}", description, newline);

        let mut act = self.players.act_with(&myself, &target);

        if !target.is_extra_description() {
            echo!(act.target(), "$^$n looks at you.\r\n");
            echo!(act.others(), "$^$n looks at $N.\r\n");
        }

        if let Some(door) = &target.components().door {
            if door.closed {
                echo!(act.myself(), "$^$E is closed.\r\n");
                return;
            } else {
                echo!(act.myself(), "$^$E is opened.\r\n");
            }
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
                    "    `S[`y{}`S]:`^ {}\r\n",
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

        let (message, emote_prefix) = if message.starts_with('[') {
            let end = message.find(']').unwrap_or_else(|| message.len());
            (&message[end + 1..], Some(&message[1..end]))
        } else {
            (&message[..], None)
        };

        let (message, emote_suffix) = if message.ends_with(']') {
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

    pub fn do_recall(&mut self, location: Option<&str>) {
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

        if let Some(door) = &exit.components().door {
            if door.closed {
                echo!(
                    self.info(),
                    "It's closed, you'll need to open it first.\r\n"
                );
                return true;
            }
        }

        let mut act = self.players.act_alone(&myself);

        let exit_keyword = exit.main_keyword();

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

        self.check_triggers_others(Action::Exit { direction });

        self.entity_world.move_entity(self.entity_id, to_room_id);

        // Reacquire everything, the acting stage is now changed.
        let myself = self.entity_world.entity_info(self.entity_id);
        let exit = self.entity_world.entity_info(exit_id);
        let mut act = self.players.act_alone(&myself);
        echo!(
            act.others(),
            "$^$n arrives from the {}.\r\n",
            opposite_direction(exit.main_keyword())
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
                exit.main_keyword(),
                exit.component_info().short_description(),
                other_room
            )
        }
    }

    pub fn do_emote(&mut self, message: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself).store_acts();

        let needs_period = message.chars().last().map(|c| c.is_alphanumeric()) == Some(true);
        let period = if needs_period { "." } else { "" };

        if message.contains('*') {
            // Allow players to suppress the space after the name (e.g. "*'s eyes shine")
            // or to put the name in a different place.
            echo!(
                act.myself(),
                "You emote: $^{}{}\r\n",
                message.replace("*", "$n"),
                period,
            );
            echo!(
                act.others(),
                "$^{}{}\r\n",
                message.replace("*", "$n"),
                period
            );
        } else {
            echo!(act.myself(), "You emote: $^$n {}{}\r\n", message, period);
            echo!(act.others(), "$^$n {}{}\r\n", message, period);
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
                echo!(
                    self.info(),
                    "You can't pmote yourself, use an untargetted '`Wemote`^' instead.\r\n"
                );
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
            echo!(
                act.myself(),
                "You emote: $^$n {}\r\n",
                message.replace("@", &you_or_target)
            );
            echo!(act.target(), "$^$n {}\r\n", message.replace("@", "you"));
            echo!(
                act.others(),
                "$^$n {}\r\n",
                message.replace("@", target_name)
            );
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
                    let mut act = self.players.act_alone(&myself).store_acts();
                    echo!(act.myself(), "{}\r\n", &social.reflected_self);
                    if !social.reflected_others.is_empty() {
                        echo!(act.others(), "{}\r\n", &social.reflected_others);
                    }
                    let acts = act.into_acts();
                    self.check_act_triggers(acts);
                }
                Found::Other(target) => {
                    let mut act = self.players.act_with(&myself, &target).store_acts();
                    echo!(act.myself(), "{}\r\n", &social.targetted_self);

                    if !social.targetted_target.is_empty() {
                        echo!(act.target(), "{}\r\n", &social.targetted_target);
                    }
                    if !social.targetted_others.is_empty() {
                        echo!(act.others(), "{}\r\n", &social.targetted_others);
                    }
                    let acts = act.into_acts();
                    self.check_act_triggers(acts);
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

    pub fn do_get_all(&mut self, forcefully: bool) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let objects: Vec<_> = myself
            .room()
            .objects()
            .filter(|object| *object != myself)
            .map(|object| object.main_keyword().to_string())
            .collect();

        for object in objects {
            self.do_get(Some(&object), forcefully);
        }
    }

    pub fn do_drop_all(&mut self, forcefully: bool) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let objects: Vec<_> = myself
            .objects()
            .map(|object| object.main_keyword().to_string())
            .collect();

        for object in objects {
            self.do_drop(Some(&object), forcefully);
        }
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
            }
        }
    }

    pub fn do_get_from(&mut self, object: &str, container: &str, forcefully: bool) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let container = myself.find_entity(container, |e| {
            let is_container = e
                .components()
                .object
                .as_ref()
                .map(|o| o.container)
                .unwrap_or(false);
            *e != myself && (forcefully || is_container)
        });

        let container = match container {
            Found::Myself | Found::WrongSelf => {
                echo!(
                    self.info(),
                    "You can't take things from yourself, you already have them!\r\n"
                );
                return;
            }
            Found::Other(other) => other,
            Found::Nothing | Found::WrongOther(_) => {
                echo!(
                    self.info(),
                    "You don't see any container named like that here.\r\n"
                );
                return;
            }
        };

        if let Some(door) = &container.components().door {
            let mut act = self.players.act_with(&myself, &container);
            if door.closed {
                echo!(
                    act.myself(),
                    "You can't get anything from $N. It's closed!\r\n"
                );
                return;
            }
        }

        let object = container.find_entity(object, |object| {
            (object.is_object() || forcefully) && object.room() == container
        });
        let object = match object {
            Found::Nothing | Found::Myself | Found::WrongSelf | Found::WrongOther(_) => {
                let mut act = self.players.act_with(&myself, &container);
                echo!(
                    act.myself(),
                    "$^$N isn't holding anything named like that.\r\n"
                );
                return;
            }
            Found::Other(other) => other,
        };

        // Taker and container perspective
        let mut act = self.players.act_with(&myself, &container).store_acts();
        echo!(act.myself(), "You get {} from $N.\r\n", object);
        echo!(act.target(), "$^$n gets {} from you.\r\n", object);
        echo!(act.others(), "$^$n gets {} from $N.\r\n", object);
        let acts = act.into_acts();

        // Object and container's inventory perspective
        let mut act = self.players.act_with(&object, &myself).store_acts();
        echo!(act.myself(), "$N picks you up from {}.\r\n", container);
        echo!(
            act.others(),
            "$^$n is picked up by $N from {}.\r\n",
            container
        );

        let object_id = object.entity_id();

        self.entity_world.move_entity(object_id, self.entity_id);
        self.check_act_triggers(acts);
    }

    pub fn do_drop(&mut self, object_name: Option<&str>, forcefully: bool) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let object_name = match object_name {
            Some(name) => name,
            None => return echo!(self.info(), "Drop what?\r\n"),
        };

        let found = myself
            .contained_entities()
            .filter_by_keyword(object_name)
            .filter_or(|e| e.is_object() || forcefully, "$^$N is not an object.")
            .filter_or(|e| e.room() == myself, "You aren't holding $N.")
            .filter_or(
                |e| *e != myself,
                "You attempt to let go of yourself, but somehow the rest of you just keeps on\r\n\
                sticking to your hand.",
            )
            .find_one_or("You aren't holding anything named like that.");

        let object = match found {
            Ok(object) => object,
            Err(err) => return self.echo_error(err),
        };

        let mut act = self.players.act_with(&myself, &object).store_acts();
        echo!(act.myself(), "You drop $N.\r\n");
        echo!(act.target(), "$^$n drops you out of $m.\r\n");
        echo!(act.others(), "$^$n drops $N.\r\n");
        let acts = act.into_acts();

        let mut act = self.players.act_alone(&object);
        echo!(act.others(), "$^$n is tossed out of here.\r\n");

        let object_id = object.entity_id();
        let room_id = self.entity_world.room_of(myself.entity_id());
        self.entity_world.move_entity(object_id, room_id);

        self.check_act_triggers(acts);
    }

    pub fn do_give(&mut self, object: &str, target: &str, forcefully: bool) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let found = myself
            .visible_entities(object)
            .filter_or(
                |e| !e.is_extra_description() || forcefully,
                "That's just a descriptive detail, you can't give that away.",
            )
            .filter_or(|e| e.is_object() || forcefully, "$^$N is not an object.")
            .filter_or(|e| e.room() == myself, "You aren't holding $N.")
            .filter_or(
                |e| *e != myself,
                "But once you do, what will your consciousness be attached to",
            )
            .find_one_or("You aren't holding anything named like that.");

        let object = match found {
            Ok(object) => object,
            Err(error) => return self.echo_error(error),
        };

        let found = myself
            .visible_entities(target)
            .filter_or(
                |e| !e.is_extra_description() || forcefully,
                "That's just a descriptive detail, you can't give something to that.",
            )
            .filter_or(
                |e| e.is_mobile() || e.is_player() || forcefully,
                "$^$N doesn't capable of receiving it.",
            )
            // FIXME: "throw {} in the air", object
            .filter_or(
                |e| *e != myself,
                "You briefly throw it in the air before catching it again.",
            )
            .find_one_or("You don't see anyone here named like that.");

        let target = match found {
            Ok(target) => target,
            Err(error) => return self.echo_error(error),
        };

        // Giver and receiver perspectives
        let mut act = self.players.act_with(&myself, &target).store_acts();
        echo!(act.myself(), "You give {} to $N.\r\n", object);
        echo!(act.target(), "$^$n gives {} to you.\r\n", object);
        echo!(act.others(), "$^$n gives {} to $N.\r\n", object);
        let acts = act.into_acts();

        // Object and inventory perspectives
        let mut act = self.players.act_with(&object, &myself).store_acts();
        echo!(act.myself(), "$^$N gives you to {}.\r\n", target);
        echo!(
            act.others(),
            "$^$n is picked up by $N and given to {}.\r\n",
            target
        );

        let object_id = object.entity_id();
        let target_id = target.entity_id();

        self.entity_world.move_entity(object_id, target_id);
        self.check_act_triggers(acts);
        self.check_triggers_target(Action::Give { object_id }, target_id);
    }

    pub fn do_put_into(&mut self, object: &str, container: &str, forcefully: bool) {
        let myself = self.entity_world.entity_info(self.entity_id);

        let object = myself.find_entity(object, |e| {
            (forcefully || e.is_object()) && e.room() == myself
        });

        let object = match object {
            Found::Myself | Found::WrongSelf => {
                echo!(
                    self.info(),
                    "But once you do, what will your consciousness be attached to?\r\n"
                );
                return;
            }
            Found::Other(other) => other,
            Found::WrongOther(other) => {
                if other.room() != myself {
                    let mut act = self.players.act_with(&myself, &other);
                    echo!(act.myself(), "But you aren't holding $N!\r\n");
                    return;
                } else if other.is_extra_description() {
                    // These don't have a good short description to refer to.
                    echo!(
                        self.info(),
                        "That's just a descriptive detail, you can't put that away.\r\n"
                    );
                    return;
                } else {
                    let mut act = self.players.act_with(&myself, &other);
                    echo!(act.myself(), "But you aren't holding $N!\r\n");
                    return;
                }
            }
            Found::Nothing => {
                echo!(
                    self.info(),
                    "You aren't holding anything named like that.\r\n"
                );
                return;
            }
        };

        let target =
            myself.find_entity(container, |e| (forcefully || e.is_object()) && *e != myself);

        let container = match target {
            Found::Myself | Found::WrongSelf => {
                echo!(
                    self.players.info(&myself),
                    "You briefly throw {} in the air before catching it again.\r\n",
                    object
                );
                return;
            }
            Found::Other(other) => other,
            Found::WrongOther(other) => {
                if other.is_extra_description() {
                    // These don't have a good short description to refer to.
                    echo!(
                        self.info(),
                        "That's just a descriptive detail, you can't give something to that.\r\n"
                    );
                    return;
                } else {
                    let mut act = self.players.act_with(&myself, &other);
                    echo!(act.myself(), "$^$N doesn't capable of receiving it.\r\n");
                    return;
                }
            }
            Found::Nothing => {
                echo!(
                    self.info(),
                    "You don't see anyone here named like that.\r\n",
                );
                return;
            }
        };

        let is_container = container
            .components()
            .object
            .as_ref()
            .map(|object| object.container)
            == Some(true);
        if !(forcefully || is_container) {
            echo!(
                self.players.info(&myself),
                "But {} is not a container!\r\n",
                container
            );
            return;
        }

        if let Some(door) = &container.components().door {
            let mut act = self.players.act_with(&myself, &container);
            if door.closed {
                echo!(
                    act.myself(),
                    "You can't put anything into $N. It's closed!\r\n"
                );
                return;
            }
        }

        // Giver and receiver perspectives
        let mut act = self.players.act_with(&myself, &container).store_acts();
        echo!(act.myself(), "You put {} into $N.\r\n", object);
        echo!(act.target(), "$^$n puts {} into you.\r\n", object);
        echo!(act.others(), "$^$n puts {} into $N.\r\n", object);
        let acts = act.into_acts();

        // Object and inventory perspectives
        let mut act = self.players.act_with(&object, &myself).store_acts();
        echo!(act.myself(), "$^$N puts you into {}.\r\n", container);
        echo!(
            act.others(),
            "$^$n is picked up by $N and put into {}.\r\n",
            container
        );

        let object_id = object.entity_id();
        let target_id = container.entity_id();

        self.entity_world.move_entity(object_id, target_id);
        self.check_act_triggers(acts);
        self.check_triggers_target(Action::Give { object_id }, target_id);
    }

    pub fn do_open(&mut self, target: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let target = long_direction(target);
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
        let acts1 = act.into_acts();
        let mut acts2 = None;

        if let Some(leads_to) = target.leads_to() {
            let other_room = self.entity_world.entity_info(leads_to);

            let mut other_exit_id = None;

            for other_exit in other_room.exits() {
                if other_exit.main_keyword() == opposite_direction(target.main_keyword()) {
                    other_exit_id = Some(other_exit.entity_id());
                    break;
                }
            }

            if let Some(other_exit_id) = other_exit_id {
                let mut other_exit = self.entity_world.entity_info_mut(other_exit_id);

                if let Some(door) = &mut other_exit.components().door {
                    if door.closed {
                        door.closed = false;

                        let other_exit = self.entity_world.entity_info(other_exit_id);
                        let mut act = self.players.act_alone(&other_exit).store_acts();
                        echo!(act.myself(), "You are opened from the other side.\r\n");
                        echo!(act.others(), "$^$n is opened from the other side.\r\n");
                        acts2 = Some(act.into_acts());
                    }
                }
            }
        }

        self.check_act_triggers(acts1);
        if let Some(acts2) = acts2 {
            self.check_act_triggers(acts2);
        }
    }

    pub fn do_close(&mut self, target: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let target = long_direction(target);
        let target = myself.find_entity(target, |entity| {
            // Prefer opened doors
            if let Some(ref door) = entity.components().door {
                !door.closed
            } else {
                false
            }
        });

        let target = match target {
            Found::Myself | Found::WrongSelf => {
                // Not necessarily impossible, but doesn't seem useful.
                echo!(
                    self.info(),
                    "You can't seem to close yourself, you'll need some assistance.\r\n"
                );
                return;
            }
            Found::Nothing => {
                echo!(
                    self.info(),
                    "You don't see anything here by that name to close.\r\n"
                );
                return;
            }
            Found::Other(other) | Found::WrongOther(other) => other,
        };

        let target_id = target.entity_id();
        let mut target = self.entity_world.entity_info_mut(target_id);

        let door = match &mut target.components().door {
            Some(door) if door.closed => Err("is already closed"),
            Some(door) => Ok(door),
            None => Err("is not something you can close"),
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

        door.closed = true;

        let myself = self.entity_world.entity_info(self.entity_id);
        let target = self.entity_world.entity_info(target_id);
        let mut act = self.players.act_with(&myself, &target).store_acts();
        echo!(act.myself(), "You close $N.\r\n");
        echo!(act.target(), "$^$n closes you.\r\n");
        echo!(act.others(), "$^$n closes $N.\r\n");
        let acts1 = act.into_acts();
        let mut acts2 = None;

        if let Some(leads_to) = target.leads_to() {
            let other_room = self.entity_world.entity_info(leads_to);

            let mut other_exit_id = None;

            for other_exit in other_room.exits() {
                if other_exit.main_keyword() == opposite_direction(target.main_keyword()) {
                    other_exit_id = Some(other_exit.entity_id());
                    break;
                }
            }

            if let Some(other_exit_id) = other_exit_id {
                let mut other_exit = self.entity_world.entity_info_mut(other_exit_id);

                if let Some(door) = &mut other_exit.components().door {
                    if !door.closed {
                        door.closed = true;

                        let other_exit = self.entity_world.entity_info(other_exit_id);
                        let mut act = self.players.act_alone(&other_exit).store_acts();
                        echo!(act.myself(), "You are closed from the other side.\r\n");
                        echo!(act.others(), "$^$n is closed from the other side.\r\n");
                        acts2 = Some(act.into_acts());
                    }
                }
            }
        }

        self.check_act_triggers(acts1);
        if let Some(acts2) = acts2 {
            self.check_act_triggers(acts2);
        }
    }

    pub fn do_unlock(&mut self, target: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let target = long_direction(target);

        let found = myself
            .visible_entities(target)
            .filter_or(
                |e| *e != myself,
                "You can't seem to unlock yourself, you'll need some assistance",
            )
            .with_component_or::<Door>("$^$N is not something you can unlock.")
            .prefer_component(|_e, door| !door.locked)
            .find_one_with_component_or("You don't see anything here by that name to unlock.");

        let (target, door) = match found {
            Ok(object) => object,
            Err(error) => return self.echo_error(error),
        };

        if let Some(key_vnum) = door.key {
            let has_key = myself
                .contained_entities()
                .any(|item| item.components().general.vnum == key_vnum);

            if !has_key {
                echo!(self.info(), "You'll need a key to unlock it!\r\n");
                return;
            }
        }

        let target_id = target.entity_id();
        let mut target = self.entity_world.entity_info_mut(target_id);

        let door = match &mut target.components().door {
            Some(door) if !door.locked => Err("is already unlocked"),
            Some(door) if !door.closed => Err("is not closed"),
            Some(door) => Ok(door),
            None => Err("is not something you can unlock"),
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

        door.locked = false;

        let myself = self.entity_world.entity_info(self.entity_id);
        let target = self.entity_world.entity_info(target_id);
        let mut act = self.players.act_with(&myself, &target).store_acts();
        echo!(act.myself(), "You unlock $N.\r\n");
        echo!(act.target(), "$^$n unlocks you.\r\n");
        echo!(act.others(), "$^$n unlocks $N.\r\n");
        let acts1 = act.into_acts();
        let mut acts2 = None;

        if let Some(leads_to) = target.leads_to() {
            let other_room = self.entity_world.entity_info(leads_to);

            let mut other_exit_id = None;

            for other_exit in other_room.exits() {
                if other_exit.main_keyword() == opposite_direction(target.main_keyword()) {
                    other_exit_id = Some(other_exit.entity_id());
                    break;
                }
            }

            if let Some(other_exit_id) = other_exit_id {
                let mut other_exit = self.entity_world.entity_info_mut(other_exit_id);

                if let Some(door) = &mut other_exit.components().door {
                    if !door.locked {
                        door.locked = false;

                        let other_exit = self.entity_world.entity_info(other_exit_id);
                        let mut act = self.players.act_alone(&other_exit).store_acts();
                        echo!(act.myself(), "You are unlocked from the other side.\r\n");
                        echo!(
                            act.others(),
                            "You hear a clicking sound as $n is unlocked.\r\n"
                        );
                        acts2 = Some(act.into_acts());
                    }
                }
            }
        }

        self.check_act_triggers(acts1);
        if let Some(acts2) = acts2 {
            self.check_act_triggers(acts2);
        }
    }

    pub fn do_lock(&mut self, _target: &str) {
        echo!(self.info(), "Locking is not yet implemented.\r\n");
    }

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

        let found = myself
            .room()
            .contained_entities()
            .filter_or(
                |e| *e != myself,
                "The only shopkeeper here is yourself.\r\n",
            )
            .with_component::<Mobile>()
            .prefer_component(|_e, mobile| mobile.shopkeeper.is_some())
            .find_one_with_component_or("You don't see anyone here.");

        let (entity, mobile) = match found {
            Ok(found) => found,
            Err(error) => return self.echo_error(error),
        };

        if let Some(shop_info) = &mobile.shopkeeper {
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

        let mut act = self.players.act_with(&myself, &target).store_acts();
        echo!(act.myself(), "You start following $N.\r\n");
        echo!(act.target(), "$^$n follows you.\r\n");
        echo!(act.others(), "$^$n starts following $N.\r\n");
        let acts = act.into_acts();

        let main_keyword = target.main_keyword().to_string();
        let mut myself = self.entity_world.entity_info_mut(self.entity_id);
        myself.components().general.following = Some(main_keyword);

        self.check_act_triggers(acts);
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
