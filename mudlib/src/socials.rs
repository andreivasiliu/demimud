use std::{collections::BTreeMap, path::Path};

use inflector::Inflector;

use crate::{file_parser::FileParser, players::Players};

pub(crate) struct Socials {
    socials: BTreeMap<String, Social>,
}

#[derive(Default)]
struct Social {
    name: String,
    social_flags: String,
    position_flags: String,

    untargetted_self: String,
    untargetted_others: String,

    targetted_self: String,
    targetted_target: String,
    targetted_others: String,

    reflected_self: String,
    reflected_others: String,
}

impl Socials {
    pub fn do_act(&self, players: &mut Players, social: &str) -> bool {
        let social = match self.socials.get(social) {
            Some(social) => social,
            None => return false,
        };

        let player = players.current_player.to_title_case();

        let message_to_self = social.untargetted_self
            .replace("$n", &player);
        let message_to_others = social.untargetted_others
            .replace("$n", &player);

        players.current().echo(&message_to_self);
        players.current().echo("\r\n");
        players.others().echo(&message_to_others);
        players.others().echo("\r\n");

        true
    }
}

pub(crate) fn load_socials(path: &Path) -> Socials {
    let contents = std::fs::read_to_string(path).unwrap();
    let mut parser = FileParser::new(&contents);

    let mut socials = BTreeMap::new();

    let mut current_social = None;

    loop {
        let key = parser.read_word();

        if key == "EOF~" {
            assert!(current_social.is_none());
            break;
        }

        if key == "END" && current_social.is_some() {
            let social: Social = current_social.take().unwrap();
            socials.insert(social.name.clone(), social);
            continue;
        }

        parser.skip_one_space();

        if current_social.is_none() {
            assert_eq!(key, "name");
            let mut new_social = Social::default();
            new_social.name = parser.read_until_tilde().to_string();
            current_social = Some(new_social);
            continue;
        }

        let social = current_social.as_mut().unwrap();

        let attribute = match key {
            "social_flags" => &mut social.social_flags,
            "position_flags" => &mut social.position_flags,
            "acts[0]" => &mut social.untargetted_self,
            "acts[1]" => &mut social.untargetted_others,
            "acts[2]" => &mut social.reflected_self,
            "acts[3]" => &mut social.reflected_others,
            "acts[4]" => &mut social.targetted_self,
            "acts[5]" => &mut social.targetted_target,
            "acts[6]" => &mut social.targetted_others,
            "acts[7]" => {
                parser.read_until_tilde();
                continue;
            }
            key => parser.panic_on_line(&format!("Unrecognized key '{}' in socials file", key)),
        };

        *attribute = parser.read_until_tilde().to_string();
    }

    Socials { socials }
}
