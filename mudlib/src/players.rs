use std::{collections::HashMap, fmt::Write};

use crate::world::Vnum;

pub(crate) struct Players {
    pub(crate) locations: HashMap<String, Vnum>,
    pub(crate) echoes: HashMap<String, String>,
    pub(crate) current_player: String,
}

pub(crate) struct CurrentPlayer<'a> {
    players: &'a mut Players,
}

pub(crate) struct OtherPlayers<'a> {
    players: &'a mut Players,
    location: Vnum,
    except: String,
}

pub(crate) struct NPC<'a> {
    players: &'a mut Players,
    location: Vnum,
}

impl Players {
    pub(crate) fn current(&mut self) -> CurrentPlayer<'_> {
        CurrentPlayer {
            players: self
        }
    }

    pub(crate) fn others(&mut self) -> OtherPlayers<'_> {
        OtherPlayers {
            location: self.locations[&self.current_player],
            except: self.current_player.clone(),
            players: self,
        }
    }

    pub(crate) fn npc(&mut self, location: Vnum) -> NPC<'_> {
        NPC {
            players: self,
            location,
        }
    }
}

impl<'a> CurrentPlayer<'a> {
    pub(crate) fn echo(&mut self, message: impl AsRef<str>) {
        let message = message.as_ref();

        if !self.players.echoes.contains_key(&self.players.current_player) {
            self.players.echoes.insert(self.players.current_player.clone(), String::new());
        }

        self.players.echoes
            .get_mut(self.players.current_player.as_str())
            .unwrap()
            .push_str(message);
    }

    pub(crate) fn change_player_location(&mut self, new_location: Vnum) {
        *self.players.locations.get_mut(&self.players.current_player).unwrap() = new_location;
    }
}

impl<'a> Write for CurrentPlayer<'a> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        if let Some(echo) = self.players.echoes.get_mut(&self.players.current_player) {
            echo.write_str(s)
        } else {
            panic!("Player {} does not have an echo buffer.", self.players.current_player);
        }
    }
}

impl<'a, T: AsRef<str>> std::ops::Add<T> for CurrentPlayer<'a> {
    type Output = CurrentPlayer<'a>;

    fn add(self, rhs: T) -> Self::Output {
        let message = rhs.as_ref();
        self.players.echoes.get_mut(&self.players.current_player).unwrap().push_str(message);
        self
    }
}

impl<'a, T: AsRef<str>> std::ops::AddAssign<T> for CurrentPlayer<'a> {
    fn add_assign(&mut self, rhs: T) {
        let message = rhs.as_ref();
        self.players.echoes.get_mut(&self.players.current_player).unwrap().push_str(message);
    }
}

impl<'a> OtherPlayers<'a> {
    pub(crate) fn echo(&mut self, message: impl AsRef<str>) {
        let message = message.as_ref();

        for (player, location) in &self.players.locations {
            if location == &self.location && player != &self.except {
                self.players.echoes.get_mut(player).unwrap().push_str(message);
            }
        }
    }
}

impl<'a> Write for OtherPlayers<'a> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        Ok(self.echo(s))
    }
}

impl<'a> NPC<'a> {
    pub(crate) fn act(&mut self, message: impl AsRef<str>) {
        let message = message.as_ref();

        for (player, location) in &self.players.locations {
            if location == &self.location {
                self.players.echoes.get_mut(player).unwrap().push_str(message);
            }
        }
    }
}
