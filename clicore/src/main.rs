use std::io::{stdout, Write};

use mudlib::{colorize, Files, WorldState};

mod files;

struct Game {
    world: WorldState,
}

impl Game {
    fn new(files: &dyn Files) -> Self {
        Self {
            world: WorldState::from_files(files),
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

        echo(&colorize(echoes));
        let mut stdout = stdout();
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

fn echo(text: &str) {
    for line in text.lines() {
        // Use platform newlines instead of telnet's "\r\n"
        println!("{}", line);
    }
}

fn main() {
    let files = files::StaticFiles;

    // Print "Welcome to DemiMUD" banner
    // Made with: figlet -f small Welcome to DemiMUD | lolcat --seed 25 --force
    echo(&files.read_file("clicore/banner.txt").unwrap());

    // Note: Convert newlines to \r\n, which makes webassembly.sh not eat up
    // empty lines sometimes.
    echo("DemiMUD is a prototype MUD engine written in Rust.\r\n\r\n");

    // Print credits for Dawn of Time areas if they are included
    if cfg!(feature = "dawn-areas") {
        echo(&colorize(&files.read_file("clicore/license.txt").unwrap()));
    }

    // Print info about DemiMUD and help pages
    echo(&colorize(&files.read_file("clicore/notice.txt").unwrap()));

    let mut game = Game::new(&files);
    game.world.add_player("You");
    game.send_echoes().unwrap();

    loop {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        let words = line.split_whitespace().collect::<Vec<_>>();

        match words[..] {
            ["who"] => {
                echo("Just you. It's a CLI after all.\r\n");
            }
            ["restart"] => {
                game = Game::new(&files);
                game.world.add_player("You");
                echo("World reloaded.\r\n");
            }
            ["exit"] | ["quit"] | ["shutdown"] => {
                echo("Bye!\r\n");
                return;
            }
            [] => game.wait_for_events(),
            ref words => {
                game.world.process_player_command("You", words);
            }
        }

        game.world.update_world();

        game.send_echoes().unwrap();
    }
}
