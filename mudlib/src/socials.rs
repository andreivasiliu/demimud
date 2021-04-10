use std::{collections::BTreeMap, path::Path};

use crate::{file_parser::FileParser, files::Files};

pub(crate) struct Socials {
    socials: BTreeMap<String, Social>,
}

#[derive(Default)]
pub(crate) struct Social {
    pub(crate) name: String,
    social_flags: String,
    position_flags: String,

    pub(crate) untargetted_self: String,
    pub(crate) untargetted_others: String,

    pub(crate) targetted_self: String,
    pub(crate) targetted_target: String,
    pub(crate) targetted_others: String,

    pub(crate) reflected_self: String,
    pub(crate) reflected_others: String,
}

impl Socials {
    pub fn list(&self) -> impl Iterator<Item = &str> {
        self.socials.keys().map(|key| key.as_str())
    }

    pub fn get(&self, name: &str) -> Option<&Social> {
        self.socials.get(name)
    }
}

pub(crate) fn load_socials(files: &dyn Files, path: &str) -> Socials {
    let contents = files.read_file(path).unwrap();
    let mut parser = FileParser::new(&contents);

    let mut socials = BTreeMap::new();

    let mut current_social = None;

    loop {
        if current_social.is_none() {
            let section = parser.read_section();

            if section == "END" {
                break;
            }

            assert_eq!(section, "SOCIAL");
        }

        let key = parser.read_word();

        if key == "End" {
            let social: Social = current_social.take().unwrap();
            socials.insert(social.name.clone(), social);
            continue;
        }

        parser.skip_one_space();

        if current_social.is_none() {
            assert_eq!(key, "Name");
            current_social = Some(Social {
                name: parser.read_until_tilde().trim_start().to_string(),
                ..Default::default()
            });
            continue;
        }

        let social = current_social.as_mut().unwrap();

        let attribute = match key {
            "CharNoArg" => &mut social.untargetted_self,
            "OthersNoArg" => &mut social.untargetted_others,
            "CharAuto" => &mut social.reflected_self,
            "OthersAuto" => &mut social.reflected_others,
            "CharFound" => &mut social.targetted_self,
            "OthersFound" => &mut social.targetted_others,
            "VictFound" => &mut social.targetted_target,
            "acts[7]" => {
                parser.read_until_tilde();
                continue;
            }
            key => parser.panic_on_line(&format!("Unrecognized key '{}' in socials file", key)),
        };

        let message = parser.read_until_tilde().trim_start();

        if message.starts_with('$') {
            *attribute = String::from("$^") + message;
        } else {
            *attribute = message.to_string();
        }
    }

    Socials { socials }
}

// Dawn-format socials; currently using Ultra-Envy socials instead
#[allow(dead_code)]
fn load_old_socials(path: &Path) -> Socials {
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
            current_social = Some(Social {
                name: parser.read_until_tilde().to_string(),
                ..Default::default()
            });
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
