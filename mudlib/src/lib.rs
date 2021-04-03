use std::{collections::BTreeMap, net::SocketAddr, panic::catch_unwind, path::Path};

use libtelnet_rs::{events::TelnetEvents, Parser};
use netcore::{EntryCode, ExitCode, NetServer, Source};
use serde::{Deserialize, Serialize};

use acting::Players;
use colors::colorize;
use commands::process_player_command;
use state::WorldState;

mod acting; // Process and output things like "$n flexes $s muscles."
mod colors; // Turn codes like "`w" into "\e[37m".
mod commands; // do_say, do_look, do_get, etc
mod components; // Types of game data (mob, obj, etc) attached to entities
mod entity; // Every object in the world and relation between objects
mod file_parser; // Dawn of Time area format parser primitives
mod import; // Convert a DoW world to EntityWorld entities
mod load; // Dawn of Time area loader
mod mapper; // Map generator
mod socials; // Load socials from socials.txt
mod state; // Main game object, glues everything together
mod world; // Read-only representation of a set of Dawn of Time areas
mod find_entities; // Primitives to help with matching and filtering entities

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

struct Game {
    world_state: Box<WorldState>,
}

impl Game {
    fn new(connection_state: &mut ConnectionState, reason: &str) -> Game {
        println!("Loading area data...");
        let world = world::load_world(Path::new("data/area"));
        println!("Loading socials.txt data...");
        let socials = socials::load_socials(Path::new("data/socials.txt"));
        println!("Importing areas into entity world...");
        let mut world_state = state::create_state(world, socials);

        for connection in connection_state.connections.values() {
            if let Some(player) = &connection.player {
                world_state.add_player(player);
            }
        }

        let mut game = Game {
            world_state: Box::new(world_state),
        };

        println!("Entity world loaded, game ready.");

        let message = format!(
            "`D[`Rsystem`D]: `WOld game {}; new game created.`^\r\n",
            reason
        );
        broadcast(&mut game, connection_state, &message);

        game
    }
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

    let mut game = Game::new(&mut connection_state, "restarted");

    send_echoes(
        net_server,
        &mut game.world_state.players,
        &mut connection_state,
    );

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
                    let world_state = &mut game.world_state;

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
                                            world_state,
                                            command_words,
                                        );
                                    }
                                    &["restart"] => {
                                        echo("Scheduled restart.\r\n");
                                        schedule_restart = true;
                                    }
                                    &["shutdown"] => {
                                        echo("Scheduled shutdown.\r\n");
                                        schedule_exit = true;
                                    }
                                    words => {
                                        let player = connection
                                            .player
                                            .as_ref()
                                            .expect("Checked in previous match arm");

                                        let old_game = catch_unwind(move || {
                                            process_player_command(
                                                &mut game.world_state,
                                                player,
                                                words,
                                            );
                                            game
                                        });

                                        game = match old_game {
                                            Ok(game) => game,
                                            Err(_err) => {
                                                // Old game's kaput, make a new one
                                                Game::new(&mut connection_state, "crashed")
                                            }
                                        };
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
            netcore::NetEvent::Tick => {
                let old_game = catch_unwind(move || {
                    game.world_state.update_world();
                    game
                });
                game = match old_game {
                    Ok(game) => game,
                    Err(_err) => {
                        // Old game's kaput, make a new one
                        Game::new(&mut connection_state, "crashed")
                    }
                };
            }
        };

        // Send all buffered output to players.
        send_echoes(
            net_server,
            &mut game.world_state.players,
            &mut connection_state,
        );

        if schedule_restart {
            for &target in connection_state.connections.keys() {
                if target == 0 || target == 1 {
                    continue;
                }
                net_server.send_bytes(&Source(target), b"\r\nServer is restarting...\r\n");
                net_server.try_flush(&Source(target));
            }
            break true;
        } else if schedule_exit {
            for &target in connection_state.connections.keys() {
                if target == 0 || target == 1 {
                    continue;
                }
                net_server.send_bytes(&Source(target), b"\r\nServer is shutting down... bye!\r\n");
                net_server.try_flush(&Source(target));
            }
            break false;
        } else {
            continue;
        }
    };

    println!("Exiting mudlib's main loop.");

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
            echo("Name set. Welcome!\r\n");
            world_state.add_player(name);
            // FIXME
            // write!(
            //     world_state.players.others(),
            //     "{} materializes from thin air.\r\n",
            //     name.to_title_case()
            // )
            // .unwrap();
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
                see the names of those who are logged in.\r\n",
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
            if let Some(player_echo) = players.player_echoes.get_mut(player.as_str()) {
                let echoes = &mut player_echo.echo_buffer;
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
                    net_server.send_bytes(&target, b"> \xFF\xF9");
                }
            }
        } else if connection.sent_command && !connection.no_prompt {
            let target = Source(*target);
            net_server.send_bytes(&target, b"> \xFF\xF9");
        }
    }

    for connection in connection_state.connections.values_mut() {
        connection.sent_command = false;
    }

    for player_echo in players.player_echoes.values_mut() {
        player_echo.echo_buffer.clear();
    }
}

fn broadcast(game: &mut Game, connection_state: &mut ConnectionState, message: &str) {
    for connection in connection_state.connections.values() {
        if let Some(player) = &connection.player {
            let echo = &mut game
                .world_state
                .players
                .player_echoes
                .get_mut(player)
                .expect("All players should have an echo buffer")
                .echo_buffer;
            echo.push_str(&colorize(message));
        }
    }
}
