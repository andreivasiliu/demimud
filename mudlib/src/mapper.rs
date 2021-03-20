use std::ops::{Index, IndexMut};

use crate::world::{Vnum, World};

#[derive(Clone, Copy)]
enum MapElement {
    Empty,
    Room(u8, u8),
    Exit(u8),
}

const EXITS: &[(u8, i8, i8, &'static str)] = &[
    // Character, row offset, column offset, name
    (b'|', -1, 0, "north"),
    (b'/', -1, 1, "northeast"),
    (b'-', 0, 1, "east"),
    (b'\\', 1, 1, "southeast"),
    (b'|', 1, 0, "south"),
    (b'/', 1, -1, "southwest"),
    (b'-', 0, -1, "west"),
    (b'\\', -1, -1, "northwest"),
];

struct RoomMap {
    rooms: Vec<Vnum>,
    rows: usize,
    columns: usize,
}

impl RoomMap {
    fn new(rows: usize, columns: usize) -> Self {
        let mut rooms = Vec::new();
        rooms.resize(rows * columns, Vnum(0));

        RoomMap {
            rooms,
            rows,
            columns,
        }
    }

    fn place_neighbors(&mut self, row: usize, column: usize, world: &World) {
        let vnum = self[(row, column)];
        let room = world.room(vnum);

        for (_, row_offset, column_offset, dir_name) in EXITS {
            if let Some(exit) = room.exits.iter().find(|e| e.name == *dir_name) {
                if !world.has_room(exit.vnum) {
                    continue;
                }
                if world.room(exit.vnum).area != room.area {
                    continue;
                }

                let out_left = row == 0 && *row_offset == -1;
                let out_top = column == 0 && *column_offset == -1;
                let out_right = row == self.rows - 1 && *row_offset == 1;
                let out_bottom = column == self.columns - 1 && *column_offset == 1;

                if out_left || out_top || out_right || out_bottom {
                    continue;
                }

                let row = (row as isize + *row_offset as isize) as usize;
                let column = (column as isize + *column_offset as isize) as usize;

                if self[(row, column)] == Vnum(0) {
                    self[(row, column)] = exit.vnum;
                    self.place_neighbors(row, column, world);
                }
            }
        }
    }
}

impl Index<(usize, usize)> for RoomMap {
    type Output = Vnum;

    fn index(&self, (row, column): (usize, usize)) -> &Self::Output {
        &self.rooms[row * self.columns + column]
    }
}

impl IndexMut<(usize, usize)> for RoomMap {
    fn index_mut(&mut self, (row, column): (usize, usize)) -> &mut Self::Output {
        &mut self.rooms[row * self.columns + column]
    }
}

pub(crate) fn make_map(world: &World, vnum: Vnum) -> String {
    let room_rows = 9;
    let room_columns = 13;

    let mid_row = 4;
    let mid_column = 6;

    let mut rooms = RoomMap::new(room_rows, room_columns);

    rooms[(mid_row, mid_column)] = vnum;
    rooms.place_neighbors(mid_row, mid_column, world);

    let map_rows = room_rows * 2 + 1;
    let map_columns = room_columns * 2 + 1;

    let mut room_map = Vec::new();
    room_map.resize(map_rows * map_columns, MapElement::Empty);

    for row in 0..room_rows {
        for column in 0..room_columns {
            let vnum = rooms[(row, column)];
            if vnum == Vnum(0) {
                continue;
            }

            let room = world.room(vnum);

            let map_position = (row * 2 + 1) * map_columns + (column * 2 + 1);

            let color = match room.sector.as_str() {
                "city" => b'S',
                "inside" => b'y',
                "field" => b'Y',
                "forest" => b'G',
                "swim" => b'B',
                "noswim" => b'b',
                "hills" => b'y',
                "desert" => b'y',
                "mountain" => b'S',
                "cave" => b'S',
                "swamp" => b'y',

                sector => {
                    dbg!(sector);
                    b'w'
                }
            };

            let room_glyph = if column == mid_column && row == mid_row {
                b'*'
            } else {
                b' '
            };

            room_map[map_position] = MapElement::Room(color, room_glyph);

            for (dir, row_offset, column_offset, dir_name) in EXITS {
                if room.exits.iter().find(|e| e.name == *dir_name).is_some() {
                    let exit_position = map_position as isize
                        + (*row_offset as isize * map_columns as isize)
                        + *column_offset as isize;
                    room_map[exit_position as usize] = MapElement::Exit(*dir);
                }
            }
        }
    }

    let mut map_string = String::new();

    for map_column in 0..map_columns {
        map_string.push_str(if map_column == 0 {
            "/"
        } else if map_column == map_columns - 1 {
            "\\\r\n"
        } else if map_column % 2 == 0 {
            "-"
        } else {
            "-+-"
        });
    }

    for map_row in 0..map_rows {
        for map_column in 0..map_columns {
            let position = map_row * map_columns + map_column;

            if map_column % 2 == 0 {
                map_string.push(match room_map[position] {
                    MapElement::Empty => ' ',
                    MapElement::Room(_, _) => 'O',
                    MapElement::Exit(dir) => dir as char,
                });
            } else {
                match room_map[position] {
                    MapElement::Empty => map_string.push_str("   "),
                    MapElement::Room(color, glyph) => {
                        map_string.push_str("`");
                        map_string.push(color as char);
                        map_string.push_str("[`B");
                        map_string.push(glyph as char);
                        map_string.push_str("`");
                        map_string.push(color as char);
                        map_string.push_str("]`^");
                    }
                    MapElement::Exit(dir) => {
                        map_string.push(' ');
                        map_string.push(dir as char);
                        map_string.push(' ');
                    }
                }
            };
        }
        map_string.push_str("\r\n");
    }

    for map_column in 0..map_columns {
        map_string.push_str(if map_column == 0 {
            "\\"
        } else if map_column == map_columns - 1 {
            "/\r\n"
        } else if map_column % 2 == 0 {
            "-"
        } else {
            "-+-"
        });
    }

    map_string
}
