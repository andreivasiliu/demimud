use std::io::Write;

use mudlib::{Files, WorldState, colorize};

struct StaticFiles;

impl Files for StaticFiles {
    fn read_file(&self, path: &str) -> Result<String, std::io::Error> {
        let contents = match path {
            "data/socials.txt" => include_str!("../../data/basic_socials.txt"),
            "data/area/arealist.txt" => "basic.are",
            "data/area/basic.are" => include_str!("../../data/basic_area.txt"),
            _ => panic!("Unknown file {}", path),
        };
        Ok(contents.to_string())
    }
}

fn main() {
    let files = StaticFiles;
    let mut world = WorldState::from_files(&files);
    world.add_player("You");

    std::io::stdout().write_all(b"Welcome to DemiMUD!\n> ").unwrap();
    std::io::stdout().flush().unwrap();

    loop {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        let words = line.split_whitespace().collect::<Vec<_>>();

        match &words[..] {
            &["who"] => {
                std::io::stdout().write_all(b"Just you. It's a CLI after all.\n").unwrap();
            },
            &["restart"] => {
                world = WorldState::from_files(&files);
                world.add_player("You");
                std::io::stdout().write_all(b"World reloaded.\n").unwrap();
            },
            &["exit"] | &["quit"] | &["shutdown"] => {
                std::io::stdout().write_all(b"Bye!\n").unwrap();
                return;
            }
            words => {
                world.process_player_command("You", words);
            }
        }

        let echoes = world.player_echoes("You").unwrap();
        std::io::stdout().write_all(colorize(echoes).as_bytes()).unwrap();
        std::io::stdout().write_all(b"> ").unwrap();
        std::io::stdout().flush().unwrap();
        echoes.clear();
    }
}
