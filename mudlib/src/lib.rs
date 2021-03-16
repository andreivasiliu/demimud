use std::{collections::BTreeMap, net::SocketAddr, path::Path};

use colors::colorize;
use libtelnet_rs::{events::TelnetEvents, Parser};
use mapper::make_map;
use players::Players;
use serde::{Deserialize, Serialize};

use netcore::{EntryCode, ExitCode, NetServer, Source};
use state::WorldState;

mod colors;
mod file_parser;
mod load;
mod mapper;
mod players;
mod socials;
mod state;
mod world;

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
    no_prompt: bool,
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

    let world = world::load_world(Path::new("mudlib/area"));
    let socials = socials::load_socials(Path::new("mudlib/socials.txt"));
    let mut world_state = state::create_state(world, socials);
    world_state.reset_world();

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
                net_server.send_bytes(
                    &new_source,
                    colorize("Set your name with '`Wname YourName`^' to log in.\r\n").as_bytes(),
                );
                net_server.send_bytes(&new_source, b"> ");

                let connection = Connection {
                    player: None,
                    address: Some(address),
                    command_buffer: String::new(),
                    sent_command: false,
                    no_prompt: false,
                };

                connection_state
                    .connections
                    .insert(new_source.0, connection);
            }
            netcore::NetEvent::Disconnected => {
                let connection = connection_state
                    .connections
                    .remove(&source.0)
                    .expect("Unregistered source");
                if let Some(player) = &connection.player {
                    println!(
                        "Player {} disconnected from {}.",
                        player,
                        connection.address.unwrap()
                    );
                } else {
                    println!("Disconnected {}", connection.address.unwrap());
                }
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

                                let mut echo = |message: &str| {
                                    net_server.send_bytes(&source, message.as_bytes());
                                };

                                match words.as_slice() {
                                    &["who"] => {
                                        echo("Players currently connected to the realm:\r\n");
                                        for (target, connection) in &connection_state.connections {
                                            if let Some(address) = &connection.address {
                                                echo(&colorize(&format!(
                                                    "{}: `M{}`^ ({})\r\n",
                                                    target,
                                                    connection.player.as_deref().unwrap_or("unset"),
                                                    address
                                                )));
                                            }
                                        }
                                    }
                                    &["exit"] => {
                                        echo("Bye!\r\n");
                                        net_server.schedule_disconnect(&source);
                                        connection.no_prompt = true;
                                    }
                                    command_words if connection.player.is_none() => {
                                        process_login_command(
                                            echo,
                                            connection,
                                            &mut world_state,
                                            command_words,
                                        );
                                    }
                                    &["restart"] => {
                                        echo("Restarting...\r\n");
                                        schedule_restart = true;
                                    }
                                    &["shutdown"] => {
                                        schedule_exit = true;
                                    }
                                    words => {
                                        process_command(&mut world_state, words);
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

fn process_login_command<F: FnMut(&str)>(
    mut echo: F,
    connection: &mut Connection,
    world_state: &mut WorldState,
    command_words: &[&str],
) -> () {
    match command_words {
        &["name", name] => {
            println!(
                "Player {} logged in from {}.",
                name,
                connection.address.as_ref().unwrap()
            );
            connection.player = Some(name.to_string());
            world_state.players.current_player.clear();
            world_state.players.current_player.push_str(name);
            world_state.add_player(name.to_string());
            world_state.players.current().echo("Name set. Welcome!\r\n");
            use inflector::Inflector;
            use std::fmt::Write;
            write!(
                world_state.players.others(),
                "{} materializes from thin air.\r\n",
                name.to_title_case()
            )
            .unwrap();
        }
        &["name", ..] => {
            echo(&colorize(
                "The '`Wname`^' command can only be used with one argument after it.\r\n",
            ));
        }
        &[] => {}
        _any_command => {
            echo(&colorize(
                "But first, who are you? Type '`Wname SomeName`^' \
                to set your name, or '`Wwho`^' to\r\n\
                see the names of who is logged in.\r\n",
            ));
        }
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
                if !connection.no_prompt {
                    if let Some(player) = &connection.player {
                        net_server.send_bytes(&target, player.as_bytes());
                    }
                    net_server.send_bytes(&target, b"> ");
                }
            }
        } else if connection.sent_command && !connection.no_prompt {
            let target = Source(*target);
            net_server.send_bytes(&target, b"> ");
        }
    }

    for (_target, connection) in &mut connection_state.connections {
        connection.sent_command = false;
    }

    for (_player, echo) in &mut players.echoes {
        echo.clear();
    }
}

fn process_command(world_state: &mut WorldState, words: &[&str]) {
    use std::fmt::Write;
    match words {
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
        &["map"] => {
            let location = world_state.players.locations[&world_state.players.current_player];
            let map = make_map(&world_state.world, location);
            world_state.players.current().echo(map);
        }
        &["exits"] => {
            world_state.do_exits();
        }
        &["recall"] => {
            world_state.do_recall(None);
        }
        &["recall", location] => {
            world_state.do_recall(Some(location));
        }
        &["socials"] => {
            world_state.do_socials(None);
        }
        &["socials", social] => {
            world_state.do_socials(Some(social));
        }
        &[direction] if world_state.do_move(direction) => (),
        &[social] if world_state.do_act(social, None) => (),
        &[social, target] if world_state.do_act(social, Some(target)) => (),
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
