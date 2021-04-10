use mudlib::Files;

pub(crate) struct StaticFiles;

#[cfg(not(feature = "dawn-areas"))]
impl Files for StaticFiles {
    fn read_file(&self, path: &str) -> Result<String, std::io::Error> {
        let contents = match path {
            "data/socials.txt" => include_str!("../../data/basic_socials.txt"),
            "data/area/arealist.txt" => "basic.are",
            "data/area/basic.are" => include_str!("../../data/basic_area.txt"),
            _ => panic!("Unknown file {}", path),
        };

        Ok(contents.to_string())
    }
}

#[cfg(feature = "dawn-areas")]
impl Files for StaticFiles {
    fn read_file(&self, path: &str) -> Result<String, std::io::Error> {
        let contents = match path {
            "data/socials.txt" => include_str!("../../data/socials.txt"),
            "data/area/arealist.txt" => include_str!("../../data/area/arealist.txt"),

            "data/area/aarislan.are" => include_str!("../../data/area/aarislan.are"),
            "data/area/briaring.are" => include_str!("../../data/area/briaring.are"),
            "data/area/brigand.are" => include_str!("../../data/area/brigand.are"),
            "data/area/burrows.are" => include_str!("../../data/area/burrows.are"),
            "data/area/courier.are" => include_str!("../../data/area/courier.are"),
            "data/area/dandtown.are" => include_str!("../../data/area/dandtown.are"),
            "data/area/darkhate.are" => include_str!("../../data/area/darkhate.are"),
            "data/area/dawn.are" => include_str!("../../data/area/dawn.are"),
            "data/area/direswmp.are" => include_str!("../../data/area/direswmp.are"),
            "data/area/dzagari.are" => include_str!("../../data/area/dzagari.are"),
            "data/area/dzagswr.are" => include_str!("../../data/area/dzagswr.are"),
            "data/area/eastinn.are" => include_str!("../../data/area/eastinn.are"),
            "data/area/fgcastle.are" => include_str!("../../data/area/fgcastle.are"),
            "data/area/fishvill.are" => include_str!("../../data/area/fishvill.are"),
            "data/area/gnomehil.are" => include_str!("../../data/area/gnomehil.are"),
            "data/area/goblin.are" => include_str!("../../data/area/goblin.are"),
            "data/area/gypsie.are" => include_str!("../../data/area/gypsie.are"),
            "data/area/hafran.are" => include_str!("../../data/area/hafran.are"),
            "data/area/halfling.are" => include_str!("../../data/area/halfling.are"),
            "data/area/haunted.are" => include_str!("../../data/area/haunted.are"),
            "data/area/hive.are" => include_str!("../../data/area/hive.are"),
            "data/area/irc.are" => include_str!("../../data/area/irc.are"),
            "data/area/kennels.are" => include_str!("../../data/area/kennels.are"),
            "data/area/links.are" => include_str!("../../data/area/links.are"),
            "data/area/markrist.are" => include_str!("../../data/area/markrist.are"),
            "data/area/mekali.are" => include_str!("../../data/area/mekali.are"),
            "data/area/mekapark.are" => include_str!("../../data/area/mekapark.are"),
            "data/area/monastery.are" => include_str!("../../data/area/monastery.are"),
            "data/area/mremvill.are" => include_str!("../../data/area/mremvill.are"),
            "data/area/mudschoo.are" => include_str!("../../data/area/mudschoo.are"),
            "data/area/ooc.are" => include_str!("../../data/area/ooc.are"),
            "data/area/orcfort.are" => include_str!("../../data/area/orcfort.are"),
            "data/area/orphan.are" => include_str!("../../data/area/orphan.are"),
            "data/area/outlands.are" => include_str!("../../data/area/outlands.are"),
            "data/area/pirship.are" => include_str!("../../data/area/pirship.are"),
            "data/area/road.are" => include_str!("../../data/area/road.are"),
            "data/area/ronar.are" => include_str!("../../data/area/ronar.are"),
            "data/area/sarsi.are" => include_str!("../../data/area/sarsi.are"),
            "data/area/slime.are" => include_str!("../../data/area/slime.are"),
            "data/area/taker.are" => include_str!("../../data/area/taker.are"),
            "data/area/training.are" => include_str!("../../data/area/training.are"),
            "data/area/trollbr.are" => include_str!("../../data/area/trollbr.are"),
            "data/area/warkeep.are" => include_str!("../../data/area/warkeep.are"),
            "data/area/wild_1.are" => include_str!("../../data/area/wild_1.are"),
            "data/area/wild_2.are" => include_str!("../../data/area/wild_2.are"),

            _ => panic!("Unknown file {}", path),
        };
        Ok(contents.to_string())
    }
}
