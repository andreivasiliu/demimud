use mudlib::Files;

pub(crate) struct StaticFiles;

#[cfg(not(feature = "dawn-areas"))]
impl Files for StaticFiles {
    fn read_file_raw(&self, path: &str) -> Result<Vec<u8>, std::io::Error> {
        let contents: &[u8] = match path {
            "clicore/banner.txt" => include_bytes!("../banner.txt"),
            "clicore/license.txt" => include_bytes!("../license.txt"),
            "clicore/notice.txt" => include_bytes!("../notice.txt"),

            "data/socials.txt" => include_bytes!("../../data/basic_socials.txt"),
            "data/area/arealist.txt" => b"basic.are",
            "data/area/basic.are" => include_bytes!("../../data/basic_area.txt"),
            _ => panic!("Unknown file {}", path),
        };

        Ok(contents.to_vec())
    }
}

#[cfg(feature = "dawn-areas")]
impl Files for StaticFiles {
    fn read_file_raw(&self, path: &str) -> Result<Vec<u8>, std::io::Error> {
        let contents: &[u8] = match path {
            "clicore/banner.txt" => include_bytes!("../banner.txt"),
            "clicore/license.txt" => include_bytes!("../license.txt"),
            "clicore/notice.txt" => include_bytes!("../notice.txt"),

            "data/socials.txt" => include_bytes!("../../data/socials.txt"),
            "data/area/arealist.txt" => include_bytes!("../../data/area/arealist.txt"),

            "data/area/aarislan.are" => include_bytes!("../../data/area/aarislan.are"),
            "data/area/briaring.are" => include_bytes!("../../data/area/briaring.are"),
            "data/area/brigand.are" => include_bytes!("../../data/area/brigand.are"),
            "data/area/burrows.are" => include_bytes!("../../data/area/burrows.are"),
            "data/area/courier.are" => include_bytes!("../../data/area/courier.are"),
            "data/area/dandtown.are" => include_bytes!("../../data/area/dandtown.are"),
            "data/area/darkhate.are" => include_bytes!("../../data/area/darkhate.are"),
            "data/area/dawn.are" => include_bytes!("../../data/area/dawn.are"),
            "data/area/direswmp.are" => include_bytes!("../../data/area/direswmp.are"),
            "data/area/dzagari.are" => include_bytes!("../../data/area/dzagari.are"),
            "data/area/dzagswr.are" => include_bytes!("../../data/area/dzagswr.are"),
            "data/area/eastinn.are" => include_bytes!("../../data/area/eastinn.are"),
            "data/area/fgcastle.are" => include_bytes!("../../data/area/fgcastle.are"),
            "data/area/fishvill.are" => include_bytes!("../../data/area/fishvill.are"),
            "data/area/gnomehil.are" => include_bytes!("../../data/area/gnomehil.are"),
            "data/area/goblin.are" => include_bytes!("../../data/area/goblin.are"),
            "data/area/gypsie.are" => include_bytes!("../../data/area/gypsie.are"),
            "data/area/hafran.are" => include_bytes!("../../data/area/hafran.are"),
            "data/area/halfling.are" => include_bytes!("../../data/area/halfling.are"),
            "data/area/haunted.are" => include_bytes!("../../data/area/haunted.are"),
            "data/area/hive.are" => include_bytes!("../../data/area/hive.are"),
            "data/area/irc.are" => include_bytes!("../../data/area/irc.are"),
            "data/area/kennels.are" => include_bytes!("../../data/area/kennels.are"),
            "data/area/links.are" => include_bytes!("../../data/area/links.are"),
            "data/area/markrist.are" => include_bytes!("../../data/area/markrist.are"),
            "data/area/mekali.are" => include_bytes!("../../data/area/mekali.are"),
            "data/area/mekapark.are" => include_bytes!("../../data/area/mekapark.are"),
            "data/area/monastery.are" => include_bytes!("../../data/area/monastery.are"),
            "data/area/mremvill.are" => include_bytes!("../../data/area/mremvill.are"),
            "data/area/mudschoo.are" => include_bytes!("../../data/area/mudschoo.are"),
            "data/area/ooc.are" => include_bytes!("../../data/area/ooc.are"),
            "data/area/orcfort.are" => include_bytes!("../../data/area/orcfort.are"),
            "data/area/orphan.are" => include_bytes!("../../data/area/orphan.are"),
            "data/area/outlands.are" => include_bytes!("../../data/area/outlands.are"),
            "data/area/pirship.are" => include_bytes!("../../data/area/pirship.are"),
            "data/area/road.are" => include_bytes!("../../data/area/road.are"),
            "data/area/ronar.are" => include_bytes!("../../data/area/ronar.are"),
            "data/area/sarsi.are" => include_bytes!("../../data/area/sarsi.are"),
            "data/area/slime.are" => include_bytes!("../../data/area/slime.are"),
            "data/area/taker.are" => include_bytes!("../../data/area/taker.are"),
            "data/area/training.are" => include_bytes!("../../data/area/training.are"),
            "data/area/trollbr.are" => include_bytes!("../../data/area/trollbr.are"),
            "data/area/warkeep.are" => include_bytes!("../../data/area/warkeep.are"),
            "data/area/wild_1.are" => include_bytes!("../../data/area/wild_1.are"),
            "data/area/wild_2.are" => include_bytes!("../../data/area/wild_2.are"),

            _ => panic!("Unknown file {}", path),
        };
        Ok(contents.to_vec())
    }
}
