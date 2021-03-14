use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Default)]
#[serde(transparent)]
pub(super) struct Vnum(pub(super) usize);

#[derive(Serialize, Deserialize, Default, Clone)]
pub(super) struct Room {
    pub(super) vnum: Vnum,
    pub(super) name: String,
    pub(super) description: String,

    #[serde(default)]
    pub(super) exits: Vec<Exit>,
    #[serde(default)]
    pub(super) room_echoes: Vec<RoomEcho>,
    #[serde(default)]
    pub(super) extra_descriptions: Vec<ExtraDescription>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(super) struct Exit {
    pub(super) name: String,
    pub(super) vnum: Vnum,
    pub(super) description: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(super) struct RoomEcho {
    pub(super) first_hour: u8,
    pub(super) last_hour: u8,
    pub(super) percentage: u16,
    pub(super) echo_text: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub(super) struct ExtraDescription {
    pub(super) keyword: String,
    pub(super) description: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub(super) struct Mobile {
    pub(super) vnum: Vnum,
    pub(super) name: String,
    pub(super) short_description: String,
    pub(super) long_description: String,
    pub(super) description: String,

    pub(super) sentinel: bool,
    pub(super) unseen: bool,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub(super) struct Object {
    pub(super) vnum: Vnum,
    pub(super) name: String,
    pub(super) short_description: String,
    pub(super) description: String,

    #[serde(default)]
    pub(super) extra_descriptions: Vec<ExtraDescription>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct AreaData {
    pub(super) name: String,
    pub(super) short_name: String,

    pub(super) vnums: (Vnum, Vnum),
    pub(super) continent: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum ResetCommand {
    Mob {
        m_num: Vnum,
        global_limit: u16,
        r_num: Vnum,
        room_limit: u16,
    },
    Object {
        o_num: Vnum,
        global_limit: i16,
        r_num: Vnum,
    },
    Door {
        r_num: Vnum,
        direction: String,
        door_flags: Vec<String>,
    },
}

#[derive(Serialize, Deserialize)]
pub(super) struct Area {
    pub(super) area_data: AreaData,

    pub(super) rooms: Vec<Room>,
    pub(super) objects: Vec<Object>,
    pub(super) mobiles: Vec<Mobile>,
    pub(super) resets: Vec<ResetCommand>,
}

#[derive(Default)]
pub(super) struct World {
    pub(super) areas: Vec<(AreaData, Vec<ResetCommand>)>,

    pub(super) rooms: Vec<Room>,
    pub(super) objects: Vec<Object>,
    pub(super) mobiles: Vec<Mobile>,
}

pub(super) fn load_world(path: &Path, area_names: &[&str]) -> World {
    let mut world = World::default();

    for file_name in area_names {
        let contents = std::fs::read_to_string(&path.join(file_name)).unwrap();
        let area = crate::load::load_area(&contents);

        world.areas.push((area.area_data, area.resets));

        for room in area.rooms {
            let vnum = room.vnum.0;
            if world.rooms.len() <= vnum {
                world.rooms.resize(vnum + 1, Room::default());
            }
            world.rooms[vnum] = room;
        }

        for object in area.objects {
            let vnum = object.vnum.0;
            if world.objects.len() <= vnum {
                world.objects.resize(vnum + 1, Object::default());
            }
            world.objects[vnum] = object;
        }

        for mobile in area.mobiles {
            let vnum = mobile.vnum.0;
            if world.mobiles.len() <= vnum {
                world.mobiles.resize(vnum + 1, Mobile::default());
            }
            world.mobiles[vnum] = mobile;
        }
    }

    world
}

impl World {
    pub(super) fn room(&self, vnum: Vnum) -> &Room {
        &self
            .rooms
            .get(vnum.0)
            .unwrap_or_else(|| panic!("Room v{} not found", vnum.0))
    }

    pub(super) fn mobile(&self, vnum: Vnum) -> &Mobile {
        &self
            .mobiles
            .get(vnum.0)
            .unwrap_or_else(|| panic!("Mobile v{} not found", vnum.0))
    }

    pub(super) fn object(&self, vnum: Vnum) -> &Object {
        &self
            .objects
            .get(vnum.0)
            .unwrap_or_else(|| panic!("Object v{} not found", vnum.0))
    }

    pub(super) fn has_room(&self, vnum: Vnum) -> bool {
        match self.rooms.get(vnum.0) {
            Some(room) => room.vnum.0 != 0,
            None => false,
        }
    }
}
