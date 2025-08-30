// Todo: Make these modules their own plugins. They don't need to be ordered between each other.
mod animation;
mod input;
mod loading;
mod multiplayer;

use crate::AppState;
use avian3d::prelude::*;
use avian3d::PhysicsPlugins;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_tnua::prelude::TnuaControllerPlugin;
use bevy_tnua::TnuaUserControlsSystemSet;
use bevy_tnua_avian3d::TnuaAvian3dPlugin;
use leafwing_input_manager::prelude::{ActionState, InputManagerPlugin};
use multiplayer::MultiplayerState;

// Sub-States
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(AppState = AppState::Overworld)]
#[states(scoped_entities)]
enum OverworldState {
    #[default]
    Loading,
    InGame,
}

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PhysicsPlugins::default(),
            PhysicsDebugPlugin::default(),
            TnuaControllerPlugin::new(FixedPreUpdate),
            TnuaAvian3dPlugin::new(FixedPreUpdate),
            InputManagerPlugin::<input::PlayerAction>::default(),
            InputManagerPlugin::<input::TextAction>::default(),
        ))
        .register_type::<input::interaction::OverworldInteraction>()
        .register_type::<animation::InitialTransform>()
        .register_type::<animation::AnimatedInteractionPromptState>()
        .register_type::<animation::AnimatedRotation>()
        .init_resource::<ActionState<input::PlayerAction>>()
        .init_resource::<ActionState<input::TextAction>>()
        .insert_resource(input::PlayerAction::default_input_map())
        .insert_resource(input::TextAction::default_input_map())
        .add_sub_state::<OverworldState>()
        .add_loading_state(
            LoadingState::new(OverworldState::Loading)
                .load_collection::<loading::LevelAssets>()
                .load_collection::<loading::SpriteAssets>()
                .load_collection::<loading::SoundAssets>()
                .load_collection::<loading::SongAssets>()
                .continue_to_state(OverworldState::InGame),
        )
        .add_observer(loading::on_add_interaction)
        .add_observer(loading::on_add_animation_prompt)
        .add_observer(loading::on_add_animated_rotation)
        .init_state::<MultiplayerState>()
        .add_event::<multiplayer::OtherPlayerMoved>()
        .add_event::<multiplayer::OtherPlayerDisconnected>()
        .add_systems(
            Startup,
            |mut text_action: ResMut<ActionState<input::TextAction>>| text_action.disable(),
        )
        .add_systems(
            OnEnter(OverworldState::InGame),
            (
                loading::setup_overworld, /*multiplayer::setup_client_runtime*/
            ),
        )
        .add_systems(
            FixedFirst,
            (
                multiplayer::read_packets.run_if(
                    in_state(MultiplayerState::Connecting).or(in_state(MultiplayerState::Online)),
                ),
                (
                    multiplayer::on_other_player_moved,
                    multiplayer::on_other_player_disconnected,
                )
                    .chain()
                    .run_if(in_state(MultiplayerState::Online)),
            )
                .chain()
                .run_if(in_state(OverworldState::InGame)),
        )
        .add_systems(
            FixedPreUpdate,
            (
                input::walk.in_set(TnuaUserControlsSystemSet),
                input::interaction::interact,
                input::interaction::proceed_text,
                input::respawn,
            )
                .chain()
                .run_if(in_state(OverworldState::InGame)),
        )
        // .add_systems(
        //     FixedUpdate,
        // )
        .add_systems(
            FixedLast,
            multiplayer::send_current_position.run_if(in_state(MultiplayerState::Online)),
        )
        .add_systems(
            Update,
            (
                follow_player_with_camera,
                animation::billboard_sprites,
                animation::animate_walk_cycles,
                animation::animate_interaction_prompts,
                animation::flicker_text_box_arrow,
                animation::oscillate_rotations,
                input::interaction::typewrite_text,
            )
                .run_if(in_state(OverworldState::InGame)),
        )
        .add_systems(
            Update,
            multiplayer::stop_client_runtime_on_window_close
                .run_if(in_state(MultiplayerState::Online)),
        );
    }
}

#[derive(Component)]
#[component(immutable)]
struct Player;

fn follow_player_with_camera(
    player_transform: Single<&Transform, With<Player>>,
    mut camera_transform: Single<&mut Transform, (With<Camera3d>, Without<Player>)>,
) {
    camera_transform.translation.x = camera_transform.translation.x.clamp(
        player_transform.translation.x - 2.0,
        player_transform.translation.x + 2.0,
    );
}
