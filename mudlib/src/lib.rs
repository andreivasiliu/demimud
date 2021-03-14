use std::{collections::BTreeMap, net::SocketAddr, path::Path};

use colors::colorize;
use libtelnet_rs::{events::TelnetEvents, Parser};
use players::Players;
use serde::{Deserialize, Serialize};

use netcore::{EntryCode, ExitCode, NetServer, Source};
use state::WorldState;

mod colors;
mod file_parser;
mod load;
mod players;
mod socials;
mod state;
mod world;
mod mapper;

#[derive(Serialize, Deserialize)]
struct ConnectionState {
    connections: BTreeMap<usize, Connection>,
}

#[derive(Serialize, Deserialize, Default)]
struct Connection {
    player: Option<String>,
    address: Option<SocketAddr>,
    command_buffer: String,
    sent_command: bool,
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

    let area_files = &["mekali.are", "mekapark.are", "links.are", "wild_1.are"];
    let world = world::load_world(Path::new("mudlib/area"), area_files);
    let socials = socials::load_socials(Path::new("mudlib/socials.txt"));
    let mut world_state = state::create_state(world, socials);
    world_state.reset_world();
    world_state.add_player("nobody".to_string());

    for connection in connection_state.connections.values() {
        if let Some(player) = &connection.player {
            world_state.add_player(player.clone());
        }
    }

    let mut pulse_mobiles = 0;

    let restart = loop {
        let mut schedule_restart = false;
        let mut schedule_exit = false;

        let (source, event) = net_server.receive_event();

        match event {
            netcore::NetEvent::Accepted(new_source, address) => {
                println!("Accepted {}", address);

                net_server.send_bytes(&new_source, b"Welcome to DemiMUD!\r\n");
                net_server.send_bytes(&new_source, b"nobody> ");

                let connection = Connection {
                    player: Some("nobody".into()),
                    address: Some(address),
                    command_buffer: Default::default(),
                    sent_command: Default::default(),
                };

                connection_state
                    .connections
                    .insert(new_source.0, connection);
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
                            let connection = connection_state
                                .connections
                                .get_mut(&source.0)
                                .expect("Unregistered connection");

                            let data = String::from_utf8_lossy(&*data);
                            let original_buffer = connection.command_buffer.len();
                            connection.command_buffer.push_str(&*data);

                            world_state.players.current_player.clear();
                            world_state
                                .players
                                .current_player
                                .push_str(connection.player.as_deref().unwrap_or("nobody"));

                            if let Some(index) = data.find('\n') {
                                // Unlike other players, this one doesn't get
                                // a newline.
                                connection.sent_command = true;

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
                                    &["who"] => {
                                        net_server.send_bytes(
                                            &source,
                                            b"Players currently connected to the realm:\r\n",
                                        );
                                        for (target, connection) in &connection_state.connections {
                                            if let Some(address) = &connection.address {
                                                net_server.send_bytes(
                                                    &source,
                                                    format!(
                                                        "{}: {} ({})\r\n",
                                                        target,
                                                        connection
                                                            .player
                                                            .as_deref()
                                                            .unwrap_or("unset"),
                                                        address
                                                    )
                                                    .as_bytes(),
                                                );
                                            }
                                        }
                                        continue;
                                    }
                                    words => {
                                        process_command(&mut world_state, connection, words);
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
            netcore::NetEvent::Tick => {
                pulse_mobiles += 1;

                if pulse_mobiles >= 4 {
                    pulse_mobiles = 0;

                    world_state.update_world();
                }
            }
        };

        // Send all buffered output to players.
        send_echoes(net_server, &mut world_state.players, &mut connection_state);

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

fn send_echoes(
    net_server: &mut NetServer,
    players: &mut Players,
    connection_state: &mut ConnectionState,
) {
    for (target, connection) in &connection_state.connections {
        if let Some(player) = &connection.player {
            if let Some(echoes) = players.echoes.get(player.as_str()) {
                if echoes.is_empty() && !connection.sent_command {
                    continue;
                }

                let target = Source(*target);

                // Send them a newline first if they didn't press enter
                if !connection.sent_command {
                    net_server.send_bytes(&target, b"\r\n");
                }

                net_server.send_bytes(&target, colorize(echoes).as_bytes());

                // Also send them a prompt
                if let Some(player) = &connection.player {
                    net_server.send_bytes(&target, player.as_bytes());
                }
                net_server.send_bytes(&target, b"> ");
            }
        }
    }

    for (_target, connection) in &mut connection_state.connections {
        connection.sent_command = false;
    }

    for (_player, echo) in &mut players.echoes {
        echo.clear();
    }
}

fn process_command(world_state: &mut WorldState, connection: &mut Connection, words: &[&str]) {
    use std::fmt::Write;
    match words {
        &["name", name] => {
            connection.player = Some(name.to_owned());
            world_state.add_player(name.to_string());
            world_state
                .players
                .current()
                .echo("Someone's soul just exited your body.\r\n");
            world_state.players.current_player.clear();
            world_state.players.current_player.push_str(name);
            world_state.players.current().echo("Name changed.\r\n");
        }
        &["look"] | &["l"] => {
            world_state.do_look();
        }
        &["look", target] | &["l", target] | &["look", "at", target] | &["l", "at", target] => {
            world_state.do_look_at(target);
        }
        &["say", ref message @ ..] => {
            world_state.do_say(&message.join(" "));
        }
        &["help"] => {
            let help_text = include_str!("../help.txt");

            world_state.players.current().echo(help_text);
        }
        &[direction] if world_state.do_move(direction) => (),
        &[social] if world_state.socials.do_act(&mut world_state.players, social) => (),
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

// fn hello_world_system() {
//     println!("hello world");
// }
