use crate::plugins::overworld::input::PlayerAction;
use crate::plugins::overworld::Player;
use bevy::prelude::{default, Single, Vec3, With};
use bevy_tnua::math::Float;
use bevy_tnua::prelude::{TnuaBuiltinWalk, TnuaController};
use leafwing_input_manager::prelude::ActionState;

// Physics Constants
const MAX_VELOCITY: Float = 4.0;
pub const FLOAT_HEIGHT: Float = 0.95;
const CLING_DISTANCE: Float = 0.1;
const SPRING_DAMPENING: Float = 1.0;
const ACCELERATION: Float = 25.0;
const COYOTE_TIME: Float = 0.0;

// Systems
pub fn apply_controls(
    query: Single<(&mut TnuaController, &ActionState<PlayerAction>), With<Player>>,
) {
    let (mut controller, action_state) = query.into_inner();

    let input = action_state.axis_pair(&PlayerAction::Walk);
    let direction = Vec3::new(input.x, 0.0, input.y);

    controller.basis(TnuaBuiltinWalk {
        desired_velocity: direction * MAX_VELOCITY,
        float_height: FLOAT_HEIGHT,
        cling_distance: CLING_DISTANCE,
        spring_dampening: SPRING_DAMPENING,
        acceleration: ACCELERATION,
        air_acceleration: ACCELERATION,
        coyote_time: COYOTE_TIME,
        ..default()
    });
}
