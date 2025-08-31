pub mod interaction;

use crate::plugins::overworld::loading::STARTING_PLAYER_TRANSLATION;
use crate::plugins::overworld::{DebugOverlayRoot, OverworldState, Player};
use avian3d::prelude::PhysicsGizmos;
use bevy::prelude::*;
use bevy_tnua::math::Float;
use bevy_tnua::prelude::{TnuaBuiltinWalk, TnuaController};
use bevy_tnua::TnuaUserControlsSystemSet;
use leafwing_input_manager::prelude::{
    ActionState, InputMap, VirtualDPad, WithDualAxisProcessingPipelineExt,
};
use leafwing_input_manager::Actionlike;

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<interaction::OverworldInteraction>()
            .init_resource::<ActionState<PlayerAction>>()
            .init_resource::<ActionState<TextAction>>()
            .insert_resource(PlayerAction::default_input_map())
            .insert_resource(TextAction::default_input_map())
            .add_systems(
                Startup,
                |mut text_action: ResMut<ActionState<TextAction>>| text_action.disable(),
            )
            .add_systems(
                FixedPreUpdate,
                (
                    walk.in_set(TnuaUserControlsSystemSet),
                    interaction::interact,
                    interaction::proceed_text,
                    respawn,
                    toggle_debug_mode,
                )
                    .chain()
                    .run_if(in_state(OverworldState::InGame)),
            )
            .add_systems(
                Update,
                interaction::typewrite_text.run_if(in_state(OverworldState::InGame)),
            );
    }
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum PlayerAction {
    #[actionlike(DualAxis)]
    Walk,
    #[actionlike(Button)]
    Interact,
    #[actionlike(Button)]
    Respawn,
    #[actionlike(Button)]
    ToggleDebugMode,
}

impl PlayerAction {
    pub fn default_input_map() -> InputMap<Self> {
        InputMap::default()
            .with_dual_axis(Self::Walk, VirtualDPad::arrow_keys().inverted_y())
            .with(Self::Interact, KeyCode::KeyZ)
            .with(Self::Respawn, KeyCode::KeyR)
            .with(Self::ToggleDebugMode, KeyCode::KeyD)
    }
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum TextAction {
    Proceed,
}

impl TextAction {
    pub fn default_input_map() -> InputMap<Self> {
        InputMap::default().with(Self::Proceed, KeyCode::KeyZ)
    }
}

// Physics Constants
const MAX_VELOCITY: Float = 4.0;
pub const FLOAT_HEIGHT: Float = 0.95;
const CLING_DISTANCE: Float = 0.1;
const SPRING_DAMPENING: Float = 0.5;
const ACCELERATION: Float = 25.0;

// Systems
pub fn walk(
    query: Single<&mut TnuaController, With<Player>>,
    player_action: Res<ActionState<PlayerAction>>,
) {
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

pub fn respawn(
    transform: Single<&mut Transform, With<Player>>,
    player_action: Res<ActionState<PlayerAction>>,
) {
    if player_action.just_pressed(&PlayerAction::Respawn) {
        let mut transform = transform.into_inner();
        transform.translation = STARTING_PLAYER_TRANSLATION;
    }
}

pub fn toggle_debug_mode(
    visibility: Single<&mut Visibility, With<DebugOverlayRoot>>,
    player_action: Res<ActionState<PlayerAction>>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    if player_action.just_pressed(&PlayerAction::ToggleDebugMode) {
        let mut visibility = visibility.into_inner();
        match *visibility {
            Visibility::Hidden => {
                *visibility = Visibility::Visible;
                *config_store.config_mut::<PhysicsGizmos>().1 = PhysicsGizmos::default();
            }
            Visibility::Visible => {
                *visibility = Visibility::Hidden;
                *config_store.config_mut::<PhysicsGizmos>().1 = PhysicsGizmos::none();
            }
            _ => panic!("Invalid visibility state for debug overlay"),
        }
    }
}
