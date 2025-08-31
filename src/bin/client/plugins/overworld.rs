mod animation;
mod input;
mod loading;
mod multiplayer;

use crate::{AppState, PetscopFont};
use avian3d::prelude::*;
use avian3d::PhysicsPlugins;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::text::LineHeight;
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
            // multiplayer::MultiplayerPlugin,
        ))
        .add_sub_state::<OverworldState>()
        .add_systems(OnEnter(OverworldState::Loading), setup)
        .add_systems(
            Update,
            (
                follow_player_with_camera.run_if(in_state(OverworldState::InGame)),
                update_debug_overlay,
            ),
        );
    }
}

#[derive(Component)]
struct DebugOverlayRoot {
    refresh_timer: Timer,
}

fn setup(mut commands: Commands, petscop_font: Res<PetscopFont>) {
    // Spawn debug overlay
    // Possible fix for overlay bugs: get entity and insert renderlayer or UITargetCamera
    commands.spawn((
        Visibility::Visible,
        DebugOverlayRoot {
            refresh_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        },
        Node {
            // We need to make sure the overlay doesn't affect the position of other UI nodes
            position_type: PositionType::Absolute,
            ..Default::default()
        },
        // Render overlay on top of everything
        GlobalZIndex(1),
        children![(
            // Index 0: FPS display
            Text::default(),
            petscop_font.clone().with_line_height(LineHeight::default()),
            TextColor::BLACK,
            children![(
                // Index 1: Coordinates display
                TextSpan::default(),
                petscop_font.clone().with_line_height(LineHeight::default()),
                TextColor::BLACK,
            )]
        )],
    ));
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

fn update_debug_overlay(
    root: Single<(&Visibility, &mut DebugOverlayRoot, &Children)>,
    time: Res<Time>,
    player_transform: Single<&Transform, With<Player>>,
    diagnostic: Res<DiagnosticsStore>,
    mut writer: TextUiWriter,
) {
    let (visibility, mut root, children) = root.into_inner();

    if visibility == Visibility::Visible {
        root.refresh_timer.tick(time.delta());
        if root.refresh_timer.just_finished() {
            let fps_span = children
                .get(0)
                .expect("The debug overlay root should have one child");
            let fps = diagnostic
                .get(&FrameTimeDiagnosticsPlugin::FPS)
                .expect("FPS should be in the diagnostic store.")
                .smoothed()
                .expect("FPS should always have an exponential moving average.");
            *writer
                .get_text(*fps_span, 0)
                .expect("The FPS text span should be present.") = format!("FPS: {:.2}", fps);

            let transform = player_transform.into_inner();
            *writer
                .get_text(*fps_span, 1)
                .expect("The coords text span should be present.") = format!(
                "\nPlayer position:\nX (Right): {}\nY (Up): {}\nZ (Backward): {}",
                transform.translation.x, transform.translation.y, transform.translation.z
            );
        }
    }
}
