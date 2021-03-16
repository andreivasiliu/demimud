use rand::random;

use crate::world::{
    Area, AreaData, Exit, ExtraDescription, Gender, Mobile, Object, ResetCommand, Room, Vnum,
};

use crate::file_parser::FileParser;

pub(super) fn load_area(area_file_contents: &str) -> Area {
    let mut parser = FileParser::new(area_file_contents);

    let mut area_data = None;
    let mut mobiles = None;
    let mut objects = None;
    let mut rooms = None;
    let mut resets = None;

    loop {
        let section = parser.read_section();

        match section {
            "$" => break,
            "DAWNAREADATA" => area_data = Some(load_area_data(&mut parser)),
            "MOBILES" => mobiles = Some(load_mobile_data(&mut parser)),
            "OBJECTS" => objects = Some(load_object_data(&mut parser)),
            "ROOMS" => rooms = Some(load_room_data(&mut parser)),
            "SPECIALS" => skip_specials(&mut parser),
            "RESETS2" => resets = Some(load_resets(&mut parser)),
            "SHOPS" => break,
            "MOBPROGS" => break,
            section => panic!("Unrecognized section: '#{}'", section),
        }
    }

    let area_data = area_data.unwrap();
    let mut rooms = rooms.unwrap();

    for room in &mut rooms {
        room.area = area_data.short_name.clone();
    }

    Area {
        area_data,
        rooms,
        objects: objects.unwrap(),
        mobiles: mobiles.unwrap(),
        resets: resets.unwrap(),
    }
}

fn load_area_data(parser: &mut FileParser) -> AreaData {
    let mut area_data = AreaData {
        name: Default::default(),
        short_name: Default::default(),
        vnums: Default::default(),
        continent: Default::default(),
    };

    loop {
        let key = parser.read_word();
        parser.skip_all_space();

        let value = match key {
            "End" | "END" => break,
            "Version" | "*parent_codebase" | "VNUMs" | "LRange" | "LComment" | "Security"
            | "colourcode" | "MapScale" | "MapLevel" | "Vnum_offset" => parser.read_until_newline(),
            "FromMUD" | "Name" | "ShortName" | "Builders" | "Credits" | "build_restricts"
            | "AFlags" | "Colour" | "Continent" | "*LastSaved" => parser.read_until_tilde(),
            section => panic!("Unrecognized area data section: '{}'", section),
        };

        match key {
            "Name" => area_data.name = value.to_string(),
            "ShortName" => area_data.short_name = value.to_string(),
            "VNUMs" => {
                let mut vnums = value.split_whitespace().map(|word| word.parse().unwrap());

                let vnum_1 = vnums.next().unwrap();
                let vnum_2 = vnums.next().unwrap();

                area_data.vnums = (Vnum(vnum_1), Vnum(vnum_2));
            }
            "Continent" => area_data.continent = value.to_string(),
            _ => (),
        }
    }

    area_data
}

fn load_mobile_data(parser: &mut FileParser) -> Vec<Mobile> {
    let mut mobiles = Vec::new();

    loop {
        let vnum = parser.read_section().parse().unwrap();

        if vnum == 0 {
            break;
        }

        mobiles.push(load_mobile(parser, vnum))
    }

    mobiles
}

fn load_mobile(parser: &mut FileParser, vnum: usize) -> Mobile {
    let mut mobile = Mobile::default();
    mobile.vnum = Vnum(vnum);

    loop {
        let key = parser.read_word();

        if key != "End" && key != "END" {
            parser.skip_one_space();
        }

        let value = match key {
            "END" | "End" => break,
            "Name" | "ShortD" | "LongD" | "Desc" | "Race" | "Act" | "Act2" | "AffBy" | "AffBy2"
            | "Off" | "Imm" | "Res" | "Vuln" | "Form" | "Part" | "StartP" | "DefPos" | "Size"
            | "Sex" | "MProg" => parser.read_until_tilde(),
            "Align" | "XPMod" | "Level" | "Hitroll" | "HitDice" | "ManaDice" | "DamDice"
            | "DamType" | "AC" | "Wealth" | "Material" | "Helpgroup" | "InnBuy" | "InnSell"
            | "InnOpen" | "InnClose" | "InnRoom" => parser.read_until_newline(),
            key => panic!("Unrecognized mobile data key: '{}'", key),
        };

        match key {
            "Name" => mobile.name = value.to_string(),
            "ShortD" => mobile.short_description = value.to_string(),
            "LongD" => mobile.long_description = value.to_string(),
            "Desc" => mobile.description = value.to_string(),
            "Sex" => {
                mobile.gender = match value.trim_start() {
                    "male" => Gender::Male,
                    "female" => Gender::Female,
                    "neutral" => Gender::Neutral,
                    "random" => {
                        if random() {
                            Gender::Male
                        } else {
                            Gender::Female
                        }
                    }
                    gender => parser.panic_on_line(&format!("Unknown sex/gender: {}", gender)),
                }
            }
            "Act" => {
                for word in value.split_whitespace() {
                    match word {
                        "dont_wander" => mobile.sentinel = true,
                        "unseen" => mobile.unseen = true,
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    }

    mobile
}

fn load_object_data(parser: &mut FileParser) -> Vec<Object> {
    let mut objects = Vec::new();

    loop {
        let vnum = parser.read_section().parse().unwrap();

        if vnum == 0 {
            break;
        }

        objects.push(load_object(parser, vnum))
    }

    objects
}

fn load_object(parser: &mut FileParser, vnum: usize) -> Object {
    let mut object = Object::default();
    object.vnum = Vnum(vnum);

    loop {
        let key = parser.read_word();

        if key != "End" && key != "END" {
            parser.skip_one_space();
        }

        let mut value2 = None;

        let value = match key {
            "END" | "End" => break,
            "Name" | "Short" | "Desc" | "ItemType" | "Material" | "Extra" | "Extra2" | "Wear"
            | "ClassAllowances" | "AttuneFlags" => parser.read_until_tilde(),
            "Level" | "Cost" | "Condition" | "Asize" | "Rsize" | "Values" | "Weight" | "Affect" => {
                parser.read_until_newline()
            }
            "ExtraDesc" => {
                value2 = Some(parser.read_until_tilde());
                parser.read_until_tilde()
            }
            key => panic!("Unrecognized object data key: '{}'", key),
        };

        match key {
            "Name" => object.name = value.to_string(),
            "Short" => object.short_description = value.to_string(),
            "Desc" => object.description = value.to_string(),
            "ExtraDesc" => object.extra_descriptions.push(ExtraDescription {
                keyword: value2.unwrap().to_string(),
                description: value.to_string(),
            }),
            _ => (),
        }
    }

    object
}

fn load_room_data(parser: &mut FileParser) -> Vec<Room> {
    let mut rooms = Vec::new();

    loop {
        let vnum = parser.read_section().parse().unwrap();

        if vnum == 0 {
            break;
        }

        rooms.push(load_room(parser, vnum))
    }

    rooms
}

fn load_room(parser: &mut FileParser, vnum: usize) -> Room {
    let mut room = Room::default();
    room.vnum = Vnum(vnum);

    loop {
        let key = parser.read_word();

        if key != "End" && key != "END" {
            parser.skip_one_space();
        }

        let mut value2 = None;

        let value = match key {
            "END" | "End" => break,
            "Name" | "Desc" | "RoomFlags" | "Sector" | "RoomEcho" | "EDesc" | "EFlags"
            | "EKeywords" => parser.read_until_tilde(),
            "Mana" | "Heal" | "LockerQuant" | "LockerInitRent" | "LockerOngoRent"
            | "LockerWeight" | "LockerCapacity" | "LockerPickProof" | "Exit" | "EKeyvnum" => {
                parser.read_until_newline()
            }
            "ExtraDesc" => {
                value2 = Some(parser.read_until_tilde());
                parser.read_until_tilde()
            }
            key => panic!("Unrecognized room data key: '{}'", key),
        };

        match key {
            "Name" => room.name = value.to_string(),
            "Desc" => room.description = value.to_string(),
            "Sector" => room.sector = value.to_string(),
            "Exit" => {
                let mut args = value.split_whitespace();
                let name = args.next().unwrap();
                let vnum = args.next().unwrap().parse().unwrap();
                room.exits.push(Exit {
                    name: name.to_string(),
                    vnum: Vnum(vnum),
                    description: None,
                })
            }
            "EDesc" => {
                let exit = room.exits.last_mut().unwrap();
                exit.description = Some(value.to_string());
            }
            "ExtraDesc" => room.extra_descriptions.push(ExtraDescription {
                keyword: value2.unwrap().to_string(),
                description: value.to_string(),
            }),
            _ => (),
        }
    }

    room
}

fn skip_specials(parser: &mut FileParser) {
    loop {
        let line = parser.read_until_newline();
        if line == "S" {
            break;
        }
    }
}

fn load_resets(parser: &mut FileParser) -> Vec<ResetCommand> {
    let mut resets = Vec::new();

    loop {
        let reset_type = parser.read_word();

        match reset_type {
            "S" => {
                parser.skip_one_newline();
                break;
            }
            "O" => {
                let zero = parser.read_word();
                let o_num = parser.read_word().parse().unwrap();
                let global_limit = parser.read_word().parse().unwrap();
                let r_num = parser.read_word().parse().unwrap();

                assert_eq!(zero, "0");

                resets.push(ResetCommand::Object {
                    o_num: Vnum(o_num),
                    global_limit,
                    r_num: Vnum(r_num),
                })
            }
            "M" => {
                let zero = parser.read_word();
                let m_num = parser.read_word().parse().unwrap();
                let global_limit = parser.read_word().parse().unwrap();
                let r_num = parser.read_word().parse().unwrap();
                let room_limit = parser.read_word().parse().unwrap();

                assert_eq!(zero, "0");

                resets.push(ResetCommand::Mob {
                    m_num: Vnum(m_num),
                    global_limit,
                    r_num: Vnum(r_num),
                    room_limit,
                })
            }
            _ => {
                parser.read_until_newline();
            }
        }
    }

    resets
}
