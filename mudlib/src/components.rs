use string_interner::StringInterner;

use crate::{entity::EntityInfo, world::{Gender, MobProgTrigger, Shop, Vnum}};

#[derive(Clone)]
pub(crate) struct IntStr {
    symbol: string_interner::symbol::SymbolU32,
}

#[derive(Clone)]
pub(crate) struct Components {
    pub act_info: ActInfo,
    pub descriptions: Descriptions,
    pub general: GeneralData,
    pub mobile: Option<Mobile>,
    pub object: Option<Object>,
    pub door: Option<Door>,
    pub mobprog: Option<MobProg>,
    pub silver: Option<Silver>,
}

#[derive(Clone)]
pub(crate) struct GeneralData {
    pub vnum: Vnum,
    pub area: String,
    pub sector: Option<String>,
    pub entity_type: EntityType,
    pub equipped: Option<String>,
    pub command_queue: Vec<(u16, String)>,
    pub following: Option<String>,
}

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Player,
    Mobile,
    Object,
    Room,
    Exit,
    ExtraDescription,
    MobProg,
}

#[derive(Clone)]
pub(crate) struct Mobile {
    pub wander: bool,
    pub shopkeeper: Option<Shop>,
    pub remember: Option<String>,
}

#[derive(Clone)]
pub(crate) struct Object {
    pub cost: i32,
    pub key: Option<Vnum>,
    pub container: bool,
    pub food: bool,
}

#[derive(Clone)]
pub(crate) struct Door {
    pub closed: bool,
    pub locked: bool,
    pub key: Option<Vnum>,
}

#[derive(Clone)]
pub(crate) struct MobProg {
    pub trigger: MobProgTrigger,
    pub code: String,
}

#[derive(Clone)]
pub(crate) struct Silver {
    pub amount: usize,
}

#[derive(Clone)]
pub(crate) struct ActInfo {
    keyword: IntStr,
    short_description: IntStr,
    gender: Gender,
}

#[derive(Clone)]
pub(crate) struct Descriptions {
    /// Internal title, seen when looking in the room inside of it (aka room title).
    /// Example: "In a forest."
    internal_title: IntStr,
    /// Internal description, seen when inside of it (aka room description).
    /// Example: "You are in a forest, and there are some trees here."
    internal: IntStr,
    /// External description, seen when looking at it from outside (aka object description).
    /// Example: "This is a very intricate object with various details that would describe it."
    external: IntStr,
    /// Lateral description, seen when looking in the room ()
    /// Example: "An object is in the room here."
    lateral: IntStr,
}

pub(crate) trait ComponentFromEntity {
    fn component_from_entity<'e>(entity: &EntityInfo<'e>) -> Option<&'e Self>;
}

impl ComponentFromEntity for Object {
    fn component_from_entity<'e>(entity: &EntityInfo<'e>) -> Option<&'e Self> {
        entity.components().object.as_ref()
    }
}

impl ComponentFromEntity for Door {
    fn component_from_entity<'e>(entity: &EntityInfo<'e>) -> Option<&'e Self> {
        entity.components().door.as_ref()
    }
}

impl ComponentFromEntity for Mobile {
    fn component_from_entity<'e>(entity: &EntityInfo<'e>) -> Option<&'e Self> {
        entity.components().mobile.as_ref()
    }
}

impl ComponentFromEntity for Shop {
    fn component_from_entity<'e>(entity: &EntityInfo<'e>) -> Option<&'e Self> {
        entity.components().mobile.as_ref().and_then(|mobile| mobile.shopkeeper.as_ref())
    }
}

pub(crate) struct EntityComponentInfo<'i, 'c> {
    interner: &'i StringInterner,
    components: &'c Components,
}

pub(crate) trait InternComponent {
    fn act_info(&mut self, keyword: &str, short_description: &str, gender: Gender) -> ActInfo;
    fn set_short_description(&mut self, act_info: &mut ActInfo, short_description: &str);
    fn descriptions(
        &mut self,
        title: &str,
        internal: &str,
        external: &str,
        lateral: &str,
    ) -> Descriptions;
}

impl InternComponent for StringInterner {
    fn act_info(&mut self, keyword: &str, short_description: &str, gender: Gender) -> ActInfo {
        let mut intern = |string| IntStr {
            symbol: self.get_or_intern(string),
        };
        ActInfo {
            keyword: intern(keyword),
            short_description: intern(short_description),
            gender: gender,
        }
    }

    fn set_short_description(&mut self, act_info: &mut ActInfo, short_description: &str) {
        // Note: old value is forever lost; this kinda leaks
        act_info.short_description = IntStr { symbol:
            self.get_or_intern(short_description)
        };
    }

    fn descriptions(
        &mut self,
        title: &str,
        internal: &str,
        external: &str,
        lateral: &str,
    ) -> Descriptions {
        let mut intern = |string| IntStr {
            symbol: self.get_or_intern(string),
        };
        Descriptions {
            internal_title: intern(title),
            internal: intern(internal),
            external: intern(external),
            lateral: intern(lateral),
        }
    }
}

impl<'i, 'c> EntityComponentInfo<'i, 'c> {
    pub fn new(components: &'c Components, interner: &'i StringInterner) -> Self {
        Self {
            interner,
            components,
        }
    }

    fn resolve(&self, string_symbol: &IntStr) -> &'i str {
        self.interner
            .resolve(string_symbol.symbol)
            .expect("All component strings should be interned")
    }

    pub fn short_description(&self) -> &'i str {
        self.resolve(&self.components.act_info.short_description)
    }

    pub fn internal_title(&self) -> &'i str {
        self.resolve(&self.components.descriptions.internal_title)
    }

    pub fn external_description(&self) -> &'i str {
        self.resolve(&self.components.descriptions.external)
    }

    pub fn internal_description(&self) -> &'i str {
        self.resolve(&self.components.descriptions.internal)
    }

    pub fn lateral_description(&self) -> &'i str {
        self.resolve(&self.components.descriptions.lateral)
    }

    pub fn gender(&self) -> Gender {
        self.components.act_info.gender
    }

    pub fn keyword(&self) -> &'i str {
        self.resolve(&self.components.act_info.keyword)
    }
}
