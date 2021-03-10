use std::{collections::BTreeMap, path::Path};

//use bevy::prelude::*;
use libtelnet_rs::{events::TelnetEvents, Parser};
use serde::{Deserialize, Serialize};

use netcore::{EntryCode, ExitCode, NetServer, Source};
use state::WorldState;

mod load;
mod state;
mod world;

#[derive(Serialize, Deserialize)]
struct ConnectionState {
    connections: BTreeMap<usize, Connection>,
}

#[derive(Default, Serialize, Deserialize)]
struct Connection {
    player: Option<String>,
    command_buffer: String,
}

#[no_mangle]
pub extern "C" fn do_things(net_server: &mut NetServer, entry_code: EntryCode) -> ExitCode {
    let mut connection_state = match entry_code {
        EntryCode::New => {
            let mut connections = BTreeMap::new();
            // Tick events and accept events are built-in as sources 0 and 1
            connections.insert(0, Connection::default());
            connections.insert(1, Connection::default());

            ConnectionState { connections }
        }
        EntryCode::Restarted { initializer } => bincode::deserialize(&*initializer).expect(""),
    };

    // App::build()
    //     .add_plugin(bevy::log::LogPlugin::default())
    //     .add_system(hello_world_system.system())
    //     .run();

    for (&target, connection) in &connection_state.connections {
        if target == 0 || target == 1 {
            continue;
        }
        net_server.send_bytes(&Source(target), b"Server is back online.\r\n");
        if let Some(player) = &connection.player {
            net_server.send_bytes(&Source(target), player.as_bytes());
        }
        net_server.send_bytes(&Source(target), b"> ");
    }

    let mut telnet_parser = Parser::new();

    let area_files = &["mekali.are"];
    let world = world::load_world(Path::new("mudlib/area"), area_files);
    let mut world_state = state::create_state(world);
    world_state.reset_world();
    world_state.add_player("nobody".to_string());

    for connection in connection_state.connections.values() {
        if let Some(player) = &connection.player {
            world_state.add_player(player.clone());
        }
    }

    let restart = loop {
        let mut schedule_restart = false;
        let mut schedule_exit = false;

        let (source, event) = net_server.receive_event();

        let connection = connection_state
            .connections
            .get_mut(&source.0)
            .expect("Unregistered connection");

        match event {
            netcore::NetEvent::Accepted(new_source, _) => {
                net_server.send_bytes(&new_source, b"Welcome to DemiMUD!\r\n");
                net_server.send_bytes(&new_source, b"> ");

                connection_state
                    .connections
                    .insert(new_source.0, Connection::default());
            }
            netcore::NetEvent::Disconnected => {
                connection_state.connections.remove(&source.0);
            }
            netcore::NetEvent::Received(bytes) => {
                for event in telnet_parser.receive(bytes) {
                    match event {
                        TelnetEvents::DataSend(data) => {
                            net_server.send_bytes(&source, &*data);
                        }
                        TelnetEvents::DataReceive(data) => {
                            let data = String::from_utf8_lossy(&*data);
                            let original_buffer = connection.command_buffer.len();
                            connection.command_buffer.push_str(&*data);

                            if let Some(index) = data.find('\n') {
                                let command: String = connection
                                    .command_buffer
                                    .drain(..original_buffer + index)
                                    .collect();

                                let words: Vec<&str> = command.split_whitespace().collect();

                                match words.as_slice() {
                                    &["restart"] => {
                                        net_server.send_bytes(&source, b"Restarting...\r\n");
                                        schedule_restart = true;
                                    }
                                    &["exit"] => {
                                        schedule_exit = true;
                                    }
                                    words => {
                                        process_command(
                                            net_server,
                                            &mut world_state,
                                            &source,
                                            connection,
                                            words,
                                        );
                                    }
                                }
                                if let Some(player) = &connection.player {
                                    net_server.send_bytes(&source, player.as_bytes());
                                }
                                net_server.send_bytes(&source, b"> ");
                            }
                        }
                        _ => (),
                    }
                }
            }
            netcore::NetEvent::Tick => {}
        };

        if schedule_restart {
            for (&target, _connection) in &connection_state.connections {
                if target == 0 || target == 1 {
                    continue;
                }
                net_server.send_bytes(&Source(target), b"\r\nServer is restarting...\r\n");
            }
            break true;
        } else if schedule_exit {
            for (&target, _connection) in &connection_state.connections {
                if target == 0 || target == 1 {
                    continue;
                }
                net_server.send_bytes(&Source(target), b"\r\nServer is shutting down...\r\n");
            }
            break false;
        } else {
            continue;
        }
    };

    println!("Done!");

    if restart {
        ExitCode::PleaseRestart {
            initializer: bincode::serialize(&connection_state)
                .unwrap()
                .into_boxed_slice(),
        }
    } else {
        ExitCode::Exit
    }
}

fn process_command(
    net_server: &mut NetServer,
    world_state: &mut WorldState,
    target: &Source,
    connection: &mut Connection,
    words: &[&str],
) -> () {
    match words {
        &["name", name] => {
            connection.player = Some(name.to_owned());
            world_state.add_player(name.to_string());
            net_server.send_bytes(target, b"Name changed.\r\n");
        }
        &["look"] | &["l"] => {
            let name = connection.player.as_deref().unwrap_or("nobody");
            let output = world_state.do_look(name);
            net_server.send_bytes(target, output.as_bytes());
        }
        &["look", object] | &["l", object] | &["look", "at", object] | &["l", "at", object] => {
            let name = connection.player.as_deref().unwrap_or("nobody");
            let output = world_state.do_look_at(name, object);
            net_server.send_bytes(target, output.as_bytes());
        }
        &["n"] | &["e"] | &["s"] | &["w"] => {
            let name = connection.player.as_deref().unwrap_or("nobody");
            let output = world_state.do_move(name, words[0]);
            net_server.send_bytes(target, output.as_bytes());
        }
        &["help"] => {
            let help_text = "Currently implemented commands:\r\n\
                * look [object] (alias: l)\r\n\
                * north, east, south, west (aliases: n, e, s, w)\r\n\
                * name <name>\r\n\
                \r\n\
                Examples:\r\n\
                * l\r\n\
                * l bench\r\n\
                * n\r\n\
                * name whyte\r\n\
                \r\n\
            ";

            net_server.send_bytes(target, help_text.as_bytes());
        }
        &[cmd_word, ..] => {
            let cmd = format!(
                "Unrecognized command: {}. Type 'help' for a list of commands.\r\n",
                cmd_word
            );
            net_server.send_bytes(target, cmd.as_bytes());
        }
        &[] => (),
    }
}

// fn hello_world_system() {
//     println!("hello world");
// }
