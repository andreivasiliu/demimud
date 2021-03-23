use string_interner::StringInterner;

use crate::world::Gender;

pub(crate) struct IntStr {
    symbol: string_interner::symbol::SymbolU32,
}

pub(crate) struct Components {
    pub act_info: ActInfo,
    pub descriptions: Descriptions,
    pub general: GeneralData,
    pub mobile: Option<Mobile>,
}

pub(crate) struct GeneralData {
    pub area: String,
    pub sector: Option<String>,
    pub entity_type: EntityType,
    pub equipped: Option<String>,
}

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Player,
    Mobile,
    Object,
    Room,
    Exit,
    ExtraDescription,
}

pub(crate) struct Mobile {
    pub wander: bool,
}

pub(crate) struct ActInfo {
    keyword: IntStr,
    short_description: IntStr,
    gender: Gender,
}

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
pub(crate) struct EntityComponentInfo<'i, 'c> {
    interner: &'i StringInterner,
    components: &'c Components,
}

pub(crate) trait InternComponent {
    fn act_info(&mut self, keyword: &str, short_description: &str, gender: Gender) -> ActInfo;
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