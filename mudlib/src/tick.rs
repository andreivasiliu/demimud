use string_interner::StringInterner;

use crate::{agent::EntityAgent, commands::process_agent_command, WorldState};

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

            let exit_symbol = interner.get_or_intern(exit.main_keyword());
            wanderers.push((entity_id, exit_symbol));
        }
    }

    for (wanderer_id, exit_symbol) in wanderers {
        let mut agent = EntityAgent {
            entity_world,
            socials: &world_state.socials,
            vnum_templates: &world_state.vnum_templates,
            areas: &world_state.areas,
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
            areas: &world_state.areas,
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
