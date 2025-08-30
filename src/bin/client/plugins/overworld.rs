mod animation;
mod input;
mod loading;
mod multiplayer;

use crate::AppState;
use avian3d::prelude::*;
use avian3d::PhysicsPlugins;
use bevy::prelude::*;
use bevy_tnua::prelude::TnuaControllerPlugin;
use bevy_tnua_avian3d::TnuaAvian3dPlugin;
use leafwing_input_manager::prelude::InputManagerPlugin;

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
        // Custom plugins
        .add_plugins((
            animation::AnimationPlugin,
            input::InputPlugin,
            loading::LoadingPlugin,
            multiplayer::MultiplayerPlugin,
        ))
        .add_sub_state::<OverworldState>()
        .add_systems(
            Update,
            follow_player_with_camera.run_if(in_state(OverworldState::InGame)),
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
