pub mod interaction;

use crate::plugins::overworld::loading::STARTING_PLAYER_TRANSLATION;
use crate::plugins::overworld::Player;
use bevy::prelude::*;
use bevy_tnua::math::Float;
use bevy_tnua::prelude::{TnuaBuiltinWalk, TnuaController};
use leafwing_input_manager::prelude::{
    ActionState, InputMap, VirtualDPad, WithDualAxisProcessingPipelineExt,
};
use leafwing_input_manager::Actionlike;

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum PlayerAction {
    #[actionlike(DualAxis)]
    Walk,
    #[actionlike(Button)]
    Interact,
    #[actionlike(Button)]
    Respawn,
}

impl PlayerAction {
    pub fn default_input_map() -> InputMap<Self> {
        InputMap::default()
            .with_dual_axis(Self::Walk, VirtualDPad::arrow_keys().inverted_y())
            .with(Self::Interact, KeyCode::KeyZ)
            .with(Self::Respawn, KeyCode::KeyR)
    }
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum TextAction {
    Proceed,
}

impl TextAction {
    pub fn default_input_map() -> InputMap<Self> {
        InputMap::default()
            .with(Self::Proceed, KeyCode::KeyZ)
    }
}

// Physics Constants
const MAX_VELOCITY: Float = 4.0;
pub const FLOAT_HEIGHT: Float = 0.95;
const CLING_DISTANCE: Float = 0.1;
const SPRING_DAMPENING: Float = 0.5;
const ACCELERATION: Float = 25.0;

// Systems
pub fn walk(query: Single<&mut TnuaController, With<Player>>, player_action: Res<ActionState<PlayerAction>>) {
    let mut controller = query.into_inner();

    let input = player_action.axis_pair(&PlayerAction::Walk);
    let direction = Vec3::new(input.x, 0.0, input.y);

    controller.basis(TnuaBuiltinWalk {
        desired_velocity: direction * MAX_VELOCITY,
        float_height: FLOAT_HEIGHT,
        cling_distance: CLING_DISTANCE,
        spring_dampening: SPRING_DAMPENING,
        acceleration: ACCELERATION,
        air_acceleration: ACCELERATION,
        coyote_time: 0.0,
        tilt_offset_angvel: 0.0,
        tilt_offset_angacl: 0.0,
        ..default()
    });
}

pub fn respawn(transform: Single<&mut Transform, With<Player>>, player_action: Res<ActionState<PlayerAction>>) {
    let mut transform = transform.into_inner();
    if player_action.just_pressed(&PlayerAction::Respawn) {
        transform.translation = STARTING_PLAYER_TRANSLATION;
    }
}