pub mod interaction;

use crate::plugins::overworld::loading::{OverworldAssetCollection, STARTING_PLAYER_TRANSLATION};
use crate::plugins::overworld::Player;
use crate::AppState;
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

// Physics Constants
const MAX_VELOCITY: Float = 4.0;
pub const FLOAT_HEIGHT: Float = 0.95;
const CLING_DISTANCE: Float = 0.1;
const SPRING_DAMPENING: Float = 0.5;
const ACCELERATION: Float = 25.0;

// Systems
pub fn walk(query: Single<(&mut TnuaController, &ActionState<PlayerAction>), With<Player>>) {
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
        coyote_time: 0.0,
        tilt_offset_angvel: 0.0,
        tilt_offset_angacl: 0.0,
        ..default()
    });
}

/// Due to the way Bevy works, this system will only run when the player has the "WithinRangeOfInteractable" component.
pub fn interact(
    action_state: Single<
        (
            &mut ActionState<PlayerAction>,
            &interaction::WithinRangeOfInteractable,
        ),
        With<Player>,
    >,
    interactions: Query<&interaction::OverworldInteraction>,
    mut commands: Commands,
    assets: Res<OverworldAssetCollection>,
) {
    let (mut action_state, within_range_of) = action_state.into_inner();
    if action_state.just_pressed(&PlayerAction::Interact) {
        let interaction = interactions
            .get(within_range_of.0)
            .expect("Interactable entities should always have interaction data");
        action_state.disable();
        match interaction {
            interaction::OverworldInteraction::Text(text) => {
                // Entire screen
                let container_node = Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                };

                let text_box_node = (
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(9.375),
                        bottom: Val::Percent(12.5),
                        width: Val::Vw(81.25),
                        height: Val::Vh(28.75),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ImageNode {
                        image: assets.sprites.text_box_image.clone(),
                        image_mode: NodeImageMode::Sliced(assets.sprites.text_box_slicer.clone()),
                        ..default()
                    },
                    interaction::TextBoxNode,
                );

                let text_node = (
                    Node {
                        width: Val::Vw(75.0),
                        height: Val::Vh(20.0),
                        ..default()
                    },
                    Text::default(),
                    interaction::CompleteText::new(text),
                );

                commands.spawn((
                    StateScoped(AppState::Overworld),
                    container_node,
                    children![(text_box_node, children![text_node])],
                ));
            }
        }
    }
}

pub fn respawn(action_state: Single<(&ActionState<PlayerAction>, &mut Transform), With<Player>>) {
    let (action_state, mut transform) = action_state.into_inner();
    if action_state.just_pressed(&PlayerAction::Respawn) {
        transform.translation = STARTING_PLAYER_TRANSLATION;
    }
}
