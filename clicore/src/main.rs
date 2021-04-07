use std::io::{stdout, Write};

use mudlib::{colorize, WorldState};

mod files;

struct Game {
    world: WorldState,
}

impl Game {
    fn new() -> Self {
        let files = files::StaticFiles;
        Self {
            world: WorldState::from_files(&files),
        }
    }

    fn echoes(&mut self) -> &mut String {
        self.world
            .player_echoes("You")
            .expect("Player was created at the start of main().")
    }

    fn send_echoes(&mut self) -> Result<(), std::io::Error> {
        let press_enter = self.world.pending_room_events("You");
        let echoes = self.echoes();

        let mut stdout = stdout();
        stdout.write_all(colorize(echoes).as_bytes())?;
        if press_enter {
            stdout.write_all(b"\x1b[1;30mpress enter")?;
        }
        stdout.write_all(b"\x1b[1;37m> \x1b[0m")?;
        stdout.flush()?;
        echoes.clear();

        Ok(())
    }

    fn wait_for_events(&mut self) {
        // If there are mobs with queued commands in the room, wait up to 100
        // ticks until something happens.
        for _ in 0..100 {
            if !self.world.pending_room_events("You") {
                return;
            }

            self.world.update_world();
            if !self.echoes().is_empty() {
                return;
            }
        }
    }
}

fn main() {
    // Print "Welcome to DemiMUD" banner
    // Made with: figlet -f small Welcome to DemiMUD | lolcat --seed 25 --force
    print!(include_str!("../banner.txt"));

    print!("DemiMUD is a prototype MUD engine written in Rust.\n\n");

    // Print credits for Dawn of Time areas if they are included
    #[cfg(feature = "dawn-areas")]
    print!("{}\n", colorize(include_str!("../license.txt")));

    // Print info about DemiMUD and help pages
    print!("{}", colorize(include_str!("../notice.txt")));

    let mut game = Game::new();
    game.world.add_player("You");
    game.send_echoes().unwrap();

    loop {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        let words = line.split_whitespace().collect::<Vec<_>>();

        match &words[..] {
            &["who"] => {
                stdout()
                    .write_all(b"Just you. It's a CLI after all.\n")
                    .unwrap();
            }
            &["restart"] => {
                game = Game::new();
                game.world.add_player("You");
                stdout().write_all(b"World reloaded.\n").unwrap();
            }
            &["exit"] | &["quit"] | &["shutdown"] => {
                stdout().write_all(b"Bye!\n").unwrap();
                return;
            }
            &[] => game.wait_for_events(),
            words => {
                game.world.process_player_command("You", words);
            }
        }

        game.world.update_world();

        game.send_echoes().unwrap();
    }
}
