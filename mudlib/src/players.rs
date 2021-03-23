use std::collections::BTreeMap;

use crate::acting::{ActingStage, Actor, TargetType};

pub(crate) struct Players {
    pub(crate) player_echoes: BTreeMap<String, PlayerEcho>,
}

#[derive(Default)]
pub(crate) struct PlayerEcho {
    pub echo_buffer: String,
    // TODO: Fix visibility
    pub current_target_type: Option<TargetType>,
}

impl Players {
    pub fn act_alone<'p, 'e>(&'p mut self, current: &'e dyn Actor) -> ActingStage<'p, 'e> {
        ActingStage::new(self, current, None)
    }

    pub fn act_with<'p, 'e>(&'p mut self, current: &'e dyn Actor, target: &'e dyn Actor) -> ActingStage<'p, 'e> {
        ActingStage::new(self, current, Some(target))
    }
}
