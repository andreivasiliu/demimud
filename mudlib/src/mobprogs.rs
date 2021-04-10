use crate::{acting::Acts, agent::EntityAgent, commands::process_agent_command, components::EntityComponentInfo, entity::{EntityId, Found}, world::{MobProgTrigger, Vnum, VnumOrKeyword}};
use crate::echo;

// Mob commands
impl<'e, 'p> EntityAgent<'e, 'p> {
    pub fn do_mob(&mut self, words: &[&str]) {
        match *words {
            ["transfer", target, to_room] => {
                self.do_mob_transfer(target, to_room);
            }
            ["dequeueall"] => {
                self.do_mob_dequeue_all();
            }
            ["at", room, ref command @ ..] => {
                self.do_mob_at(room, command);
            }
            ["goto", room] => {
                self.do_mob_goto(room);
            }
            ["mload", m_vnum] => {
                self.do_mob_mload(m_vnum);
            }
            ["oload", o_vnum] => {
                self.do_mob_oload(o_vnum);
            }
            ["call", p_vnum, target] => {
                self.do_mob_call(p_vnum, target);
            }
            ["remember", target] => {
                self.do_mob_remember(target);
            }
            ["rsay", ref message @ ..] => {
                self.do_mob_rsay(&message.join(" "));
            }
            ["echo", ref message @ ..] => {
                self.do_mob_echo(&message.join(" "));
            }
            ["vforce", target, ref command @ ..] => {
                self.do_mob_vforce(target, command);
            }
            ["force", target, ref command @ ..] => {
                // No difference from normal command
                self.do_force(target, command);
            }
            ["silent", ref command @ ..] => {
                self.do_mob_silent(command);
            }
            ["mpfollow", target] => {
                // No difference from normal command
                self.do_follow(target);
            }
            [cmd_word, ..] => {
                self.do_unknown(cmd_word);
            }
            [] => {
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
            .vnum_to_room_entity
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

        let from_room_id = target.room().entity_id();

        self.entity_world.move_entity(target_id, room_id);

        let target = self.entity_world.entity_info(target_id);
        let mut act = self.players.act_alone(&target);
        echo!(
            act.others(),
            "$^$n appears into the room out of thin air.\r\n"
        );

        // Also teleport followers.
        let mut agent = self.switch_agent(target_id);
        agent.do_look();
        agent.check_followers(from_room_id, "void", room_id);
        agent.check_triggers_others(Action::Greet);
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
            .vnum_to_room_entity
            .get(at_room_vnum)
            .and_then(|permanent_id| *permanent_id)
            .and_then(|permanent_id| self.entity_world.old_entity(&permanent_id));

        let room_id = match room {
            Some(entity) => entity.entity_id(),
            None => {
                echo!(
                    self.info(),
                    "Room '{}' does not exist or was destroyed.\r\n",
                    at_room
                );
                return;
            }
        };

        let original_room_id = myself.room().entity_id();

        self.entity_world.move_entity(self.entity_id, room_id);
        process_agent_command(self, commands);
        self.entity_world
            .move_entity(self.entity_id, original_room_id);
    }

    pub fn do_mob_goto(&mut self, to_room: &str) {
        let to_room_vnum: usize = match to_room.parse() {
            Ok(vnum) => vnum,
            Err(_) => {
                echo!(self.info(), "Room '{}' is not a valid vnum.\r\n", to_room);
                return;
            }
        };

        let room = self
            .vnum_templates
            .vnum_to_room_entity
            .get(to_room_vnum)
            .and_then(|permanent_id| *permanent_id)
            .and_then(|permanent_id| self.entity_world.old_entity(&permanent_id));

        let room = match room {
            Some(room) => room,
            None => {
                echo!(
                    self.info(),
                    "Room '{}' does not exist or was destroyed.\r\n",
                    to_room
                );
                return;
            }
        };

        let room_id = room.entity_id();
        self.entity_world.move_entity(self.entity_id, room_id);

        echo!(self.info(), "You moved to room v{}.\r\n", to_room);
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
            "Spawned $N from mobile template with vnum '{}'.\r\n",
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

    pub fn do_mob_oload(&mut self, o_vnum: &str) {
        let o_vnum: usize = match o_vnum.parse() {
            Ok(vnum) => vnum,
            Err(_) => {
                echo!(self.info(), "Vnum '{}' is not a valid number.\r\n", o_vnum);
                return;
            }
        };

        let object_components = self
            .vnum_templates
            .object_components
            .get(o_vnum)
            .and_then(|components| components.as_ref());

        let (object_components, extra_descriptions) = match object_components {
            Some(components) => components,
            None => {
                echo!(
                    self.info(),
                    "Object template with vnum '{}' does not exist.\r\n",
                    o_vnum
                );
                return;
            }
        };

        let object_id = self
            .entity_world
            .insert_entity(self.entity_id, object_components.clone());
        for extra_description_components in extra_descriptions {
            self.entity_world
                .insert_entity(object_id, extra_description_components.clone());
        }

        let myself = self.entity_world.entity_info(self.entity_id);
        let object = self.entity_world.entity_info(object_id);
        let mut act = self.players.act_with(&myself, &object);
        echo!(
            act.myself(),
            "Created $N from object template with vnum '{}'.\r\n",
            o_vnum
        );
        echo!(
            act.target(),
            "You have been created by $n into existence. Welcome!\r\n"
        );

        let mut act = self.players.act_with(&object, &myself);
        echo!(
            act.others(),
            "$^$n creates $N from thin air and plops it into here.\r\n"
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

    pub fn do_mob_echo(&mut self, message: &str) {
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);

        echo!(act.myself(), "You echo: {}\r\n", message);
        echo!(act.others(), "{}\r\n", message);
    }

    /// Same as 'mob force', but instead of a name a vnum is used.
    ///
    /// Ideally this would loop through the room's entities to find one with
    /// that vnum, but for now it just turns that vnum into a name and calls
    /// the original 'force'.
    pub fn do_mob_vforce(&mut self, m_vnum: &str, command: &[&str]) {
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

        if let Some((mobile_components, _mobprogs)) = mobile_components {
            let component_info =
                EntityComponentInfo::new(mobile_components, &self.entity_world.interner);
            let keyword = component_info.keyword();
            let main_keyword = keyword
                .split_whitespace()
                .last()
                .expect("Split gives at least one element");
            let main_keyword = main_keyword.to_string();

            self.do_force(&main_keyword, command);
        } else {
            echo!(
                self.info(),
                "You don't know of any mobiles with that vnum.\r\n"
            );
        }
    }

    pub fn do_mob_silent(&mut self, command: &[&str]) {
        // It has to suffice for now...
        let myself = self.entity_world.entity_info(self.entity_id);
        let mut act = self.players.act_alone(&myself);
        echo!(
            act.myself(),
            "You pretend to do the next thing silently.\r\n"
        );
        echo!(
            act.others(),
            "$^$n pretends to do $s next action silently.\r\n"
        );

        process_agent_command(self, command);
    }
}

pub(crate) enum Action<'a> {
    /// A word or phrase is said
    Speech { message: &'a str },

    /// Someone enters your room
    Greet,

    /// Someone exited your room
    Exit { direction: &'a str },

    /// You entered a new room
    Entry,

    /// You spawned or recalled here
    Login,

    /// You gave an object to someone
    Give { object_id: EntityId },
}

impl<'e, 'p> EntityAgent<'e, 'p> {
    pub fn check_triggers_self(&mut self, action: Action<'_>) {
        let mut triggered = Vec::new();
        let myself = self.entity_world.entity_info(self.entity_id);
        for item in myself.contained_entities() {
            if let Some(mobprog) = &item.components().mobprog {
                if let (Action::Entry, MobProgTrigger::Entry { chance }) = (&action, &mobprog.trigger) {
                    if random_percent(*chance) {
                        triggered.push(mobprog.code.clone());
                    }
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

    pub fn check_triggers_target(&mut self, action: Action<'_>, target_id: EntityId) {
        let mut triggered = Vec::new();
        let myself = self.entity_world.entity_info(self.entity_id);
        let target = self.entity_world.entity_info(target_id);

        for item in target.contained_entities() {
            if let Some(mobprog) = &item.components().mobprog {
                if let (Action::Give { object_id }, MobProgTrigger::Give { item_vnum }) = (&action, &mobprog.trigger) {
                    let object = self.entity_world.entity_info(*object_id);
                    let object_matches = match item_vnum {
                        VnumOrKeyword::Vnum(vnum) => object.components().general.vnum == *vnum,
                        VnumOrKeyword::Keyword(keyword) => object
                            .component_info()
                            .keyword()
                            .split_whitespace()
                            .any(|word| word == keyword),
                    };
                    if object.is_object() && object_matches {
                        triggered.push(mobprog.code.clone());
                    }
                }
            }
        }

        if triggered.is_empty() {
            return;
        }

        let self_keyword = myself.main_keyword().to_string();

        for code in triggered {
            let mut agent = self.switch_agent(target_id);
            agent.run_mobprog(code, self_keyword.clone());
        }
    }

    pub fn check_triggers_others(&mut self, action: Action<'_>) {
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
                            if message.contains(pattern) {
                                triggered.push((entity.entity_id(), mobprog.code.clone()));
                            }
                        }
                        (Action::Greet, MobProgTrigger::Greet { chance }) => {
                            if random_percent(*chance) {
                                triggered.push((entity.entity_id(), mobprog.code.clone()));
                            }
                        }
                        (
                            Action::Exit { direction: dir1 },
                            MobProgTrigger::Exit { direction: dir2 },
                        ) => {
                            if dir1 == dir2 {
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

        if !myself.is_player() {
            return;
        }

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
                        if line.contains(pattern) {
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

            match words[..] {
                ["if", ref condition @ ..] => {
                    accept_commands = match *condition {
                        ["room", target, "==", vnum] => {
                            assert!(
                                ["$i", "$I"].contains(&target),
                                "Don't know how to handle other targets"
                            );

                            let vnum: usize = vnum.parse().expect("Invalid vnum in mobprog");

                            let myself = self.entity_world.entity_info(self.entity_id);
                            let room = myself.room();

                            vnum == room.components().general.vnum.0
                        }
                        ["objhere", vnum] => {
                            let objname = match vnum.parse() {
                                Ok(vnum) => VnumOrKeyword::Vnum(Vnum(vnum)),
                                Err(_) => VnumOrKeyword::Keyword(vnum.to_string()),
                            };

                            let mut found = false;

                            for item in myself.room().objects() {
                                let matches_item = match objname {
                                    VnumOrKeyword::Vnum(vnum) => {
                                        vnum == item.components().general.vnum
                                    }
                                    VnumOrKeyword::Keyword(ref keyword) => item
                                        .component_info()
                                        .keyword()
                                        .split_whitespace()
                                        .any(|word| word == keyword),
                                };
                                found = found || matches_item;
                            }

                            found
                        }
                        ["carries", target, object] => {
                            let target =
                                myself.find_entity(target, |e| e.is_mobile() || e.is_player());
                            let target = match target {
                                Found::Myself => Some(myself),
                                Found::Other(other) => Some(other),
                                Found::WrongSelf | Found::WrongOther(_) | Found::Nothing => None,
                            };

                            if let Some(target) = target {
                                target.objects().any(|o| {
                                    o.component_info()
                                        .keyword()
                                        .split_whitespace()
                                        .any(|word| word == object)
                                })
                            } else {
                                false
                            }
                        }
                        // Silly, but temporary until I get a check_condition function
                        ["!carries", target, object] => {
                            let target =
                                myself.find_entity(target, |e| e.is_mobile() || e.is_player());
                            let target = match target {
                                Found::Myself => Some(myself),
                                Found::Other(other) => Some(other),
                                Found::WrongSelf | Found::WrongOther(_) | Found::Nothing => None,
                            };

                            !if let Some(target) = target {
                                target.objects().any(|o| {
                                    o.component_info()
                                        .keyword()
                                        .split_whitespace()
                                        .any(|word| word == object)
                                })
                            } else {
                                false
                            }
                        }
                        // FIXME: Wrong, but, I don't know how it can be one, ever
                        ["isnpc", _target] => true,
                        ["istarget", target] => target == remembered,
                        ["!istarget", target] => target != remembered,
                        _ => false,
                    };
                }
                ["else"] => accept_commands = !accept_commands,
                ["endif"] => accept_commands = true,
                ["end"] if accept_commands => break,
                ref command if accept_commands => {
                    process_agent_command(self, command);
                }
                _ => (),
            };
        }
    }
}

fn random_percent(chance: u8) -> bool {
    rand::random::<u32>() % 100 < chance.into()
}
