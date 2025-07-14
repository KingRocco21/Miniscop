use bevy::prelude::Reflect;
use leafwing_input_manager::prelude::{InputMap, VirtualDPad, WithDualAxisProcessingPipelineExt};
use leafwing_input_manager::Actionlike;

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum PlayerAction {
    #[actionlike(DualAxis)]
    Walk,
}

impl PlayerAction {
    pub fn default_input_map() -> InputMap<Self> {
        InputMap::default().with_dual_axis(Self::Walk, VirtualDPad::arrow_keys().inverted_y())
    }
}
