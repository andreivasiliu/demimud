use std::fmt::{Display, Formatter, Result, Write};

use crate::entity::EntityInfo;
use crate::{players::Players, world::Gender};

pub trait Actor {
    fn is_player(&self, player_name: &str) -> bool;
    fn colocated_with_player(&self, player_name: &str) -> bool;

    fn short_description(&self, f: &mut Formatter, capitalized: bool) -> Result;
    fn pronouns(&self, capitalized: bool) -> (&str, &str, &str);

    fn subjective_pronoun(&self, f: &mut Formatter, capitalized: bool) -> Result {
        self.pronouns(capitalized).0.fmt(f)
    }

    fn objective_pronoun(&self, f: &mut Formatter, capitalized: bool) -> Result {
        self.pronouns(capitalized).1.fmt(f)
    }

    fn possessive_pronoun(&self, f: &mut Formatter, capitalized: bool) -> Result {
        self.pronouns(capitalized).2.fmt(f)
    }
}

impl<'e> Actor for EntityInfo<'e> {
    fn is_player(&self, player_name: &str) -> bool {
        self.is_player(player_name)
    }

    fn colocated_with_player(&self, player_name: &str) -> bool {
        self.colocated_with_player(player_name)
    }

    fn short_description(&self, f: &mut Formatter, capitalized: bool) -> Result {
        if !capitalized {
            self.short_description().fmt(f)
        } else {
            let mut short_description = self.short_description();

            if let Some(first_character) = short_description.chars().next() {
                first_character.to_uppercase().fmt(f)?;
                short_description = &short_description[first_character.len_utf8()..];
            }

            short_description.fmt(f)
        }
    }

    fn pronouns(&self, capitalized: bool) -> (&str, &str, &str) {
        if !capitalized {
            match self.gender() {
                Gender::Male => ("he", "him", "his"),
                Gender::Female => ("she", "her", "her"),
                Gender::Neutral => ("it", "it", "its"),
            }
        } else {
            match self.gender() {
                Gender::Male => ("He", "Him", "His"),
                Gender::Female => ("She", "Her", "Her"),
                Gender::Neutral => ("It", "It", "Its"),
            }
        }
    }
}

pub(crate) struct ActingStage<'p, 'e> {
    players: &'p mut Players,

    current_actor: &'e dyn Actor,
    target_actor: Option<&'e dyn Actor>,
}

impl<'p, 'e> ActingStage<'p, 'e> {
    pub fn new(
        players: &'p mut Players,
        current: &'e dyn Actor,
        target: Option<&'e dyn Actor>,
    ) -> ActingStage<'p, 'e> {
        for (player_name, player_echo) in players.player_echoes.iter_mut() {
            player_echo.current_target_type = if current.is_player(player_name) {
                Some(TargetType::Myself)
            } else if target.map(|target| target.is_player(player_name)) == Some(true) {
                Some(TargetType::Target)
            } else if current.colocated_with_player(player_name) {
                Some(TargetType::Others)
            } else {
                None
            };
        }

        ActingStage {
            players,
            current_actor: current,
            target_actor: target,
        }
    }

    #[allow(dead_code)]
    pub fn myself<'a>(&'a mut self) -> ActTarget<'a, 'p, 'e> {
        ActTarget {
            stage: self,
            target_type: TargetType::Myself,
        }
    }

    #[allow(dead_code)]
    pub fn target<'a>(&'a mut self) -> ActTarget<'a, 'p, 'e> {
        ActTarget {
            stage: self,
            target_type: TargetType::Target,
        }
    }

    #[allow(dead_code)]
    pub fn others<'a>(&'a mut self) -> ActTarget<'a, 'p, 'e> {
        ActTarget {
            stage: self,
            target_type: TargetType::Others,
        }
    }
}

#[derive(PartialEq, Eq)]
pub(crate) enum TargetType {
    Myself,
    Target,
    Others,
}

pub(crate) struct ActTarget<'a, 'p: 'a, 'e: 'a> {
    stage: &'a mut ActingStage<'p, 'e>,
    target_type: TargetType,
}

impl Write for ActTarget<'_, '_, '_> {
    fn write_str(&mut self, message: &str) -> Result {
        for player_echo in self.stage.players.player_echoes.values_mut() {
            if player_echo.current_target_type.as_ref() == Some(&self.target_type) {
                let message = ReplaceActVariables {
                    current: self.stage.current_actor,
                    target: self.stage.target_actor,
                    message,
                };
                write!(player_echo.echo_buffer, "{}", message)?;
            }
        }

        Ok(())
    }
}

pub(crate) struct EscapeVariables<'m>(pub &'m str);

impl Display for EscapeVariables<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut message = self.0;
        
        loop {
            let var_index = message.find('$');

            match var_index {
                None => {
                    return message.fmt(f);
                },
                Some(0) => (),
                Some(index) => {
                    message[..index].fmt(f)?;
                    message = &message[index..];
                }
            }

            "$$".fmt(f)?;
            message = &message[1..];
        }
    }
}

struct ReplaceActVariables<'e, 'm> {
    current: &'e dyn Actor,
    target: Option<&'e dyn Actor>,
    message: &'m str,
}

impl Display for ReplaceActVariables<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut message = self.message;
        let mut capitalized = false;

        loop {
            let var_index = message.find('$');

            match var_index {
                None => {
                    return message.fmt(f);
                },
                Some(0) => (),
                Some(index) => {
                    capitalized = false;
                    message[..index].fmt(f)?;
                    message = &message[index..];
                }
            }

            match message[1..].chars().next() {
                Some('$') => {
                    '$'.fmt(f)?;
                }
                Some('n') => {
                    self.current.short_description(f, capitalized)?;
                }
                Some('e') => {
                    self.current.subjective_pronoun(f, capitalized)?;
                }
                Some('m') => {
                    self.current.objective_pronoun(f, capitalized)?;
                }
                Some('s') => {
                    self.current.possessive_pronoun(f, capitalized)?;
                }
                Some('N') if self.target.is_some() => {
                    let target = self.target.expect("Checked above");
                    target.short_description(f, capitalized)?;
                }
                Some('E') if self.target.is_some() => {
                    let target = self.target.expect("Checked above");
                    target.subjective_pronoun(f, capitalized)?;
                }
                Some('M') if self.target.is_some() => {
                    let target = self.target.expect("Checked above");
                    target.objective_pronoun(f, capitalized)?;
                }
                Some('S') if self.target.is_some() => {
                    let target = self.target.expect("Checked above");
                    target.possessive_pronoun(f, capitalized)?;
                }
                Some('^') => {
                    capitalized = true;
                    message = &message[2..];
                    continue;
                }
                Some(c) => {
                    '$'.fmt(f)?;
                    c.fmt(f)?;
                    message = &message[1 + c.len_utf8()..];
                    continue;
                },
                None => {
                    '$'.fmt(f)?;
                    break;
                }
            }
            capitalized = false;
            message = &message[2..];
        }

        Ok(())
    }
}

#[macro_export]
macro_rules! echo {
    ($dst:expr, $($arg:tt)*) => {{
        use std::fmt::Write;
        ($dst.write_fmt(std::format_args!($($arg)*))).expect("Write to String buffer should be infallible");
    }};
}
