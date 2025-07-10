use crate::plugins::overworld::animation::AnimationDirection;
use bevy::prelude::{default, ButtonInput, KeyCode, Res, Single, Vec3};
use bevy_tnua::math::Float;
use bevy_tnua::prelude::{TnuaBuiltinJump, TnuaBuiltinWalk, TnuaController};

// Physics Constants
const MAX_VELOCITY: Float = 4.0;
const FLOAT_HEIGHT: Float = 0.95;
const CLING_DISTANCE: Float = 0.1;
const SPRING_DAMPENING: Float = 1.0;
const ACCELERATION: Float = 25.0;
const AIR_ACCELERATION: Float = ACCELERATION;
const COYOTE_TIME: Float = 0.0;

// Systems
pub fn apply_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    query: Single<(&mut TnuaController, &mut AnimationDirection)>,
) {
    let (mut controller, mut animation_direction) = query.into_inner();

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::ArrowUp) {
        direction -= Vec3::Z;
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        direction += Vec3::Z;
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        direction -= Vec3::X;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        direction += Vec3::X;
    }
    direction = direction.clamp(Vec3::NEG_ONE, Vec3::ONE);
    animation_direction.0 = direction;

    controller.basis(TnuaBuiltinWalk {
        desired_velocity: direction * MAX_VELOCITY,
        float_height: FLOAT_HEIGHT,
        cling_distance: CLING_DISTANCE,
        spring_dampening: SPRING_DAMPENING,
        acceleration: ACCELERATION,
        air_acceleration: AIR_ACCELERATION,
        coyote_time: COYOTE_TIME,
        ..default()
    });

    if keyboard.pressed(KeyCode::Space) {
        controller.action(TnuaBuiltinJump {
            height: 1.0,
            ..default()
        });
    }
}
