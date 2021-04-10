//! Dawn of Time area loader.
//!
//! This module uses the basic primitives in `crate::file_parser` to read area
//! files, parse rooms/mobiles/objects from them, and convert them into the
//! plain object types from `crate::world`.

use rand::random;

use crate::world::{
    Area, AreaData, Exit, ExtraDescription, Gender, MobProg, MobProgTrigger, Mobile, Object,
    ResetCommand, Room, Shop, Vnum, ObjectFlags, VnumOrKeyword
};

use crate::file_parser::FileParser;

pub(super) fn load_area(area_file_contents: &str) -> Area {
    let mut parser = FileParser::new(area_file_contents);

    let mut area_data = None;
    let mut mobiles = None;
    let mut objects = None;
    let mut rooms = None;
    let mut resets = None;
    let mut shops = None;
    let mut mobprogs = None;

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
            "SHOPS" => shops = Some(load_shops(&mut parser)),
            "MOBPROGS" => mobprogs = Some(load_mobprogs(&mut parser)),
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
        shops: shops.unwrap(),
        mobprogs: mobprogs.unwrap(),
    }
}

fn load_area_data(parser: &mut FileParser) -> AreaData {
    let mut area_data = AreaData {
        name: Default::default(),
        short_name: Default::default(),
        vnums: Default::default(),
        credits: Default::default(),
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
            "Credits" => area_data.credits = value.to_string(),
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
    let mut mobile = Mobile {
        vnum: Vnum(vnum),
        .. Default::default()
    };

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
            "MProg" => {
                let mut words = value.split_whitespace();

                let (vnum, trigger) = match words.next().unwrap() {
                    "SPEECH" => (
                        words.next(),
                        MobProgTrigger::Speech {
                            pattern: words.collect::<Vec<_>>().join(" "),
                        },
                    ),
                    "RANDOM" => (
                        words.next(),
                        MobProgTrigger::Random {
                            chance: words.next().unwrap().parse().unwrap(),
                        },
                    ),
                    "DEATH" => (
                        words.next(),
                        MobProgTrigger::Death {
                            chance: words
                                .next()
                                .map(|word| if word == "all" { "100" } else { word })
                                .unwrap()
                                .parse()
                                .unwrap(),
                        },
                    ),
                    "EXIT" | "EXALL" => (
                        words.next(),
                        MobProgTrigger::Exit {
                            direction: words.next().unwrap().to_string(),
                        },
                    ),
                    "HOUR" => (
                        words.next(),
                        MobProgTrigger::Hour {
                            hour: words.next().unwrap().parse().unwrap(),
                        },
                    ),
                    "GREET" | "GRALL" => (
                        words.next(),
                        MobProgTrigger::Greet {
                            chance: words.next().unwrap().parse().unwrap(),
                        },
                    ),
                    "GIVE" => {
                        let mopprog_vnum = words.next();
                        let item = words.next().unwrap();
                        let item_vnum = if let Ok(vnum) = item.parse() {
                            VnumOrKeyword::Vnum(Vnum(vnum))
                        } else {
                            VnumOrKeyword::Keyword(item.to_string())
                        };

                        (
                            mopprog_vnum,
                            MobProgTrigger::Give {
                                item_vnum,
                            },
                        )
                    }
                    "ACT" => (
                        words.next(),
                        MobProgTrigger::Act {
                            pattern: words.collect::<Vec<_>>().join(" ").to_string(),
                        },
                    ),
                    "BRIBE" => (
                        words.next(),
                        MobProgTrigger::Bribe {
                            amount: words.next().unwrap().parse().unwrap(),
                        },
                    ),
                    "KILL" => (
                        words.next(),
                        MobProgTrigger::Kill {
                            chance: words.next().unwrap().parse().unwrap(),
                        },
                    ),
                    "ENTRY" => (
                        words.next(),
                        MobProgTrigger::Entry {
                            chance: words.next().unwrap().parse().unwrap(),
                        },
                    ),
                    "LOGINROOM" => (words.next(), MobProgTrigger::LoginRoom {}),
                    "REPOP" | "COMMAND" | "SAYTO" | "TICK" | "FIGHT" | "HPCNT" | "DELAY"
                    | "PREKILL" | "LOGOUTROOM" | "LOGINAREA" | "ROOMDEATH" => continue,
                    trigger => panic!("Unknown mobprog trigger: {}", trigger),
                };

                let vnum = Vnum(vnum.unwrap().parse().unwrap());

                mobile.mobprog_triggers.push((trigger, vnum));
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
    let mut object = Object {
        vnum: Vnum(vnum),
        .. Default::default()
    };

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
            "Cost" => object.cost = value.parse().expect("Invalid cost"),
            "Desc" => object.description = value.to_string(),
            "ItemType" => object.item_type = value.to_string(),
            "Values" => {
                if object.item_type == "container" {
                    let mut values = value.split_whitespace();
                    let _ignored = values.next();
                    let flags = values.next().expect("Second value missing");

                    let mut closable = false;
                    let mut closed = false;
                    let mut locked = false;

                    for char in flags.chars() {
                        match char {
                            'A' => closable = true,
                            'C' => closed = true,
                            'D' => locked = true,
                            _ => (),
                        };
                    }

                    object.flags = ObjectFlags::Container {
                        closable,
                        closed,
                        locked,
                    }
                }
            }
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
    let mut room = Room {
        vnum: Vnum(vnum),
        .. Default::default()
    };

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
                    ..Default::default()
                })
            }
            "EDesc" => {
                let exit = room.exits.last_mut().unwrap();
                exit.description = Some(value.to_string());
            }
            "EFlags" => {
                let exit = room.exits.last_mut().unwrap();

                for flag in value.split_whitespace() {
                    match flag {
                        "door" => exit.has_door = true,
                        "closed" => exit.is_closed = true,
                        "locked" => exit.is_locked = true,
                        _ => (),
                    }
                }
            }
            "EKeyvnum" => {
                use std::convert::TryInto;

                let exit = room.exits.last_mut().unwrap();
                let vnum: i32 = value.parse().unwrap();
                // Skip it if it's -1
                if let Ok(vnum) = vnum.try_into() {
                    exit.key = Some(Vnum(vnum));
                }
            }
            "EKeywords" => {
                let exit = room.exits.last_mut().unwrap();
                exit.extra_keywords = Some(value.to_string());
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
            "G" => {
                let zero = parser.read_word();
                let o_num = parser.read_word().parse().unwrap();
                let global_limit = parser.read_word().parse().unwrap();

                assert_eq!(zero, "0");

                resets.push(ResetCommand::Give {
                    o_num: Vnum(o_num),
                    global_limit,
                })
            }
            "E" => {
                let zero = parser.read_word();
                let o_num = parser.read_word().parse().unwrap();
                let global_limit = parser.read_word().parse().unwrap();
                let mut location = parser.read_word().to_string();

                assert_eq!(location.pop(), Some('~'));

                assert_eq!(zero, "0");

                resets.push(ResetCommand::Equip {
                    o_num: Vnum(o_num),
                    global_limit,
                    location,
                })
            }
            "P" => {
                let zero = parser.read_word();
                let o_num = parser.read_word().parse().unwrap();
                let global_limit = parser.read_word().parse().unwrap();
                let c_num = parser.read_word().parse().unwrap();
                let container_limit = parser.read_word().parse().unwrap();

                assert_eq!(zero, "0");

                resets.push(ResetCommand::Put {
                    o_num: Vnum(o_num),
                    global_limit,
                    c_num: Vnum(c_num),
                    container_limit,
                })
            }
            _ => {
                parser.read_until_newline();
            }
        }
    }

    resets
}

fn load_shops(parser: &mut FileParser) -> Vec<Shop> {
    let mut shops = Vec::new();

    loop {
        let vnum = parser.read_section().parse().unwrap();

        if vnum == 0 {
            break;
        }

        shops.push(load_shop(parser, vnum))
    }

    shops
}

fn load_shop(parser: &mut FileParser, vnum: usize) -> Shop {
    let mut shop = Shop {
        vnum: Vnum(vnum),
        buy_types: Vec::new(),
        sell_types: Vec::new(),
        profit_buy: 100,
        profit_sell: 100,
        open_hour: 0,
        close_hour: 24,
    };

    loop {
        let key = parser.read_word();

        match key {
            "buy_type" => shop.buy_types.push(parser.read_until_tilde().to_string()),
            "sell_type" => shop.buy_types.push(parser.read_until_tilde().to_string()),
            "open_hour" => {
                shop.open_hour = parser
                    .read_until_newline()
                    .trim_start()
                    .parse()
                    .expect("Open hour")
            }
            "close_hour" => {
                shop.close_hour = parser
                    .read_until_newline()
                    .trim_start()
                    .parse()
                    .expect("Close hour")
            }
            "profit_buy" => {
                shop.profit_buy = parser
                    .read_until_newline()
                    .trim_start()
                    .parse()
                    .unwrap_or_else(|string| {
                        parser.panic_on_line(&format!("Profit error for {}: '{}'", vnum, string))
                    })
            }
            "profit_sell" => {
                shop.profit_sell = parser
                    .read_until_newline()
                    .trim_start()
                    .parse()
                    .expect("Profit sell")
            }
            "END" => break,
            key => parser.panic_on_line(&format!("Unknown shop key {}", key)),
        }
    }

    shop
}

fn load_mobprogs(parser: &mut FileParser) -> Vec<MobProg> {
    let mut mobprogs = Vec::new();

    loop {
        let vnum = parser.read_section().parse().unwrap();

        if vnum == 0 {
            break;
        }

        mobprogs.push(load_mobprog(parser, vnum))
    }

    mobprogs
}

fn load_mobprog(parser: &mut FileParser, vnum: usize) -> MobProg {
    let mut title = None;
    let mut code = None;
    let mut disabled = None;

    loop {
        let key = parser.read_word();

        if key == "END" {
            break;
        }

        parser.skip_one_space();

        match key {
            "title" => title = Some(parser.read_until_tilde().to_string()),
            "code" => code = Some(parser.read_until_tilde().to_string()),
            "disabled" => disabled = Some(parser.read_until_newline()),
            key => parser.panic_on_line(&format!("Unknown mobprog key {}", key)),
        }
    }

    MobProg {
        vnum: Vnum(vnum),
        title: title.unwrap_or_else(|| "<untitled>".to_string()),
        code: code.unwrap_or_else(|| "".to_string()),
        disabled: disabled.expect("Needs disabled") != "true",
    }
}
