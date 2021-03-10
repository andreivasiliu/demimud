use crate::world::{Area, AreaData, Exit, ExtraDescription, Mobile, Object, ResetCommand, Room, Vnum};

struct AreaParser<'a>(&'a str, &'a str);

impl<'a> AreaParser<'a> {
    fn panic_on_line(&self, message: &str) -> ! {
        let bytes_read = self.1.len() - self.0.len();
        let processed_slice = &self.1[0..bytes_read];
        let lines = processed_slice.chars().filter(|c| *c == '\n').count();
        let columns = processed_slice.chars().rev().take_while(|c| *c != '\n').count();
        let last_line = &processed_slice[processed_slice.len() - columns..];

        panic!("On line {}, column {}: {}\nLast line:\n{}\n", lines+1, columns, message, last_line);
    }

    fn read_section(&mut self) -> &'a str {
        let start = self.0.find(|c:char| !c.is_whitespace()).unwrap();
        let end = self.0[start..].find(|c: char| c.is_whitespace()).unwrap();

        if &self.0[start..start+1] != "#" {
            self.panic_on_line(&format!("Expected '#', got '{}'", &self.0[start..start+1]))
        }

        let mut section = &self.0[start + 1..start + end];
        if section.chars().last() == Some('\r') {
            section = &section[..section.len() - 1]
        }
        self.0 = &self.0[start + end + 1..];
        self.skip_one_newline();
        section
    }

    fn read_word(&mut self) -> &'a str {
        let start = self.0.find(|c: char| !c.is_ascii_whitespace()).unwrap();
        let end = self.0[start..]
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap();

        let section = &self.0[start..start + end];
        self.0 = &self.0[start + end..];
        section
    }

    fn skip_one_newline(&mut self) {
        if &self.0[..2] == "\r\n" {
            self.0 = &self.0[2..];
        } else if &self.0[..1] == "\n" {
            self.0 = &self.0[1..];
        } else {
            self.panic_on_line("No newline found to skip");
        }
    }

    fn skip_one_space(&mut self) {
        if &self.0[..1] != " " {
            self.panic_on_line(&format!("Expected ' ', got '{}'", &self.0[..1]))
        }
        self.0 = &self.0[1..];
    }

    fn skip_all_space(&mut self) {
        let start = self.0.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(0);
        self.0 = &self.0[start..];
    }

    fn read_until_newline(&mut self) -> &'a str {
        let start = 0;
        let end = self.0[start..].find(|c: char| c == '\n' || c == '\r').unwrap();

        let section = &self.0[start..start + end];
        self.0 = &self.0[start + end..];
        self.skip_one_newline();
        section
    }

    fn read_until_tilde(&mut self) -> &'a str {
        let start = 0;
        let end = self.0[start..].find(|c: char| c == '~').unwrap();

        let section = &self.0[start..start + end];
        self.0 = &self.0[start + end + 1 + 1..];
        self.skip_one_newline();
        section
    }
}

pub(super) fn load_area(area_file_contents: &str) -> Area {
    let mut parser = AreaParser(area_file_contents, area_file_contents);

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

    Area {
        area_data: area_data.unwrap(),
        rooms: rooms.unwrap(),
        objects: objects.unwrap(),
        mobiles: mobiles.unwrap(),
        resets: resets.unwrap(),
    }
}

fn load_area_data(parser: &mut AreaParser) -> AreaData {
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
            "Version" | "*parent_codebase" | "VNUMs" | "LComment" | "Security" | "colourcode"
            | "MapScale" | "MapLevel" | "Vnum_offset" => parser.read_until_newline(),
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

fn load_mobile_data(parser: &mut AreaParser) -> Vec<Mobile> {
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

fn load_mobile(parser: &mut AreaParser, vnum: usize) -> Mobile {
    let mut mobile = Mobile::default();
    mobile.vnum = Vnum(vnum);

    loop {
        let key = parser.read_word();

        if key != "End" && key != "END" {
            parser.skip_one_space();
        }

        let value = match key {
            "END" | "End" => break,
            "Name" | "ShortD" | "LongD" | "Desc" | "Race" | "Act" | "AffBy" | "Off" | "Imm" | "Res" | "Vuln" | "Form" | "Part" | "StartP" | "DefPos" | "Size" | "Sex" | "MProg" => parser.read_until_tilde(),
            "Align" | "XPMod" | "Level" | "Hitroll" | "HitDice" | "ManaDice" | "DamDice" | "DamType" | "AC" | "Wealth" | "Material" => parser.read_until_newline(),
            key => panic!("Unrecognized mobile data key: '{}'", key),
        };

        match key {
            "Name" => mobile.name = value.to_string(),
            "ShortD" => mobile.short_description = value.to_string(),
            "LongD" => mobile.long_description = value.to_string(),
            "Desc" => mobile.description = value.to_string(),
            _ => (),
        }
    }

    mobile
}

fn load_object_data(parser: &mut AreaParser) -> Vec<Object> {
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

fn load_object(parser: &mut AreaParser, vnum: usize) -> Object {
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
            "Name" | "Short" | "Desc" | "ItemType" | "Material" | "Extra" | "Wear" | "ClassAllowances" => parser.read_until_tilde(),
            "Level" | "Cost" | "Condition" | "Asize" | "Rsize" | "Values" | "Weight" | "Affect" => parser.read_until_newline(),
            "ExtraDesc" => { value2 = Some(parser.read_until_tilde()); parser.read_until_tilde() }
            key => panic!("Unrecognized object data key: '{}'", key),
        };

        match key {
            "Name" => object.name = value.to_string(),
            "Short" => object.short_description = value.to_string(),
            "Desc" => object.description = value.to_string(),
            "ExtraDesc" => {
                object.extra_descriptions.push(ExtraDescription {
                    keyword: value2.unwrap().to_string(),
                    description: value.to_string(),
                })
            }
            _ => (),
        }
    }

    object
}

fn load_room_data(parser: &mut AreaParser) -> Vec<Room> {
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

fn load_room(parser: &mut AreaParser, vnum: usize) -> Room {
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
            "Name" | "Desc" | "RoomFlags" | "Sector" | "RoomEcho" | "EDesc" | "EFlags" | "EKeyvnum" | "EKeywords" => parser.read_until_tilde(),
            "Mana" | "Heal" | "LockerQuant" | "LockerInitRent" | "LockerOngoRent" | "LockerWeight" | "LockerCapacity" | "LockerPickProof" | "Exit" => parser.read_until_newline(),
            "ExtraDesc" => { value2 = Some(parser.read_until_tilde()); parser.read_until_tilde() }
            key => panic!("Unrecognized room data key: '{}'", key),
        };

        match key {
            "Name" => room.name = value.to_string(),
            "Desc" => room.description = value.to_string(),
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
            "ExtraDesc" => {
                room.extra_descriptions.push(ExtraDescription {
                    keyword: value2.unwrap().to_string(),
                    description: value.to_string(),
                })
            }
            _ => (),
        }
    }

    room
}

fn skip_specials(parser: &mut AreaParser) {
    loop {
        let line = parser.read_until_newline();
        if line == "S" {
            break;
        }
    }
}

fn load_resets(parser: &mut AreaParser) -> Vec<ResetCommand> {
    let mut resets = Vec::new();

    loop {
        let reset_type = parser.read_word();

        match reset_type {
            "S" => {
                parser.skip_one_newline();
                break;
            },
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
