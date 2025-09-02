mod animation;
mod input;
mod loading;
mod multiplayer;

use crate::AppState::Overworld;
use crate::{AppState, PetscopFont};
use avian3d::prelude::*;
use avian3d::PhysicsPlugins;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::text::LineHeight;
use bevy_asset_loader::prelude::{DynamicAssets, StandardDynamicAsset};
use bevy_tnua::prelude::TnuaControllerPlugin;
use bevy_tnua_avian3d::TnuaAvian3dPlugin;
use leafwing_input_manager::prelude::InputManagerPlugin;

// Sub-States
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(AppState = AppState::Overworld)]
#[states(scoped_entities)]
enum OverworldState {
    #[default]
    LoadingOverworld,
    LoadingLevel,
    LoadingScene,
    InScene,
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
        .add_systems(OnEnter(OverworldState::LoadingOverworld), setup)
        .add_systems(
            Update,
            (
                follow_player_with_camera.run_if(in_state(OverworldState::InScene)),
                update_debug_periodically,
                update_debug,
                update_debug_scene_data.run_if(in_state(OverworldState::InScene)),
            ),
        );
    }
}

#[derive(Component)]
struct DebugOverlayRoot {
    fps_update_timer: Timer,
}

fn setup(
    mut commands: Commands,
    petscop_font: Res<PetscopFont>,
    mut dynamic_assets: ResMut<DynamicAssets>,
) {
    // Spawn debug overlay
    // Possible fix for overlay bugs: get entity and insert renderlayer or UITargetCamera
    commands.spawn((
        StateScoped(Overworld),
        Visibility::Visible,
        DebugOverlayRoot {
            fps_update_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        },
        Node {
            // We need to make sure the overlay doesn't affect the position of other UI nodes
            position_type: PositionType::Absolute,
            ..Default::default()
        },
        // Render overlay on top of everything
        // (1 == screen transition)
        GlobalZIndex(2),
        children![(
            // Index 0: FPS display
            Text::default(),
            petscop_font.clone().with_line_height(LineHeight::default()),
            TextColor::BLACK,
            children![
                (
                    // Index 1: Coordinates display
                    TextSpan::default(),
                    petscop_font.clone().with_line_height(LineHeight::default()),
                    TextColor::BLACK,
                ),
                (
                    // Index 2: State display
                    TextSpan::default(),
                    petscop_font.clone().with_line_height(LineHeight::default()),
                    TextColor::BLACK,
                )
            ]
        )],
    ));

    // Set initial level to be loaded before entering OverworldState::LoadingLevel
    dynamic_assets.register_asset(
        "gltf",
        Box::new(StandardDynamicAsset::File {
            path: String::from("overworld/3d/Gift_Plane.glb"),
        }),
    );
    dynamic_assets.register_asset(
        "music",
        Box::new(StandardDynamicAsset::File {
            path: String::from("overworld/music/Gift_Plane.ogg"),
        }),
    );
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

    camera_transform.translation.z = camera_transform.translation.z.clamp(
        player_transform.translation.z + 7.0,
        player_transform.translation.z + 11.0,
    );
}

fn update_debug_periodically(
    root: Single<(&Visibility, &mut DebugOverlayRoot, &Children)>,
    time: Res<Time>,
    diagnostic: Res<DiagnosticsStore>,
    mut writer: TextUiWriter,
) {
    let (visibility, mut root, children) = root.into_inner();

    if visibility == Visibility::Visible {
        root.fps_update_timer.tick(time.delta());
        if root.fps_update_timer.just_finished() {
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
        }
    }
}

fn update_debug(
    root: Single<(&Visibility, &Children), With<DebugOverlayRoot>>,
    player_physics: Single<(&Transform, &LinearVelocity), With<Player>>,
    mut writer: TextUiWriter,
) {
    let (visibility, children) = root.into_inner();

    if visibility == Visibility::Visible {
        let fps_span = children
            .get(0)
            .expect("The debug overlay root should have one child");

        let (transform, velocity) = player_physics.into_inner();
        *writer
            .get_text(*fps_span, 1)
            .expect("The coords text span should be present.") = format!(
            "\nPlayer position:\n  X (Right): {:.3}\n  Y (Up): {:.3}\n  Z (Backward): {:.3}\nPlayer velocity:\n  X (Right): {:.3}\n  Y (Up): {:.3}\n  Z (Backward): {:.3}",
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            velocity.x,
            velocity.y,
            velocity.z
        );
    }
}

fn update_debug_scene_data(
    root: Single<(&Visibility, &Children), With<DebugOverlayRoot>>,
    level: Res<loading::LevelAssets>,
    scene: Res<loading::CurrentScene>,
    entities: Query<Entity>,
    mut writer: TextUiWriter,
) {
    let (visibility, children) = root.into_inner();

    if visibility == Visibility::Visible {
        let fps_span = children
            .get(0)
            .expect("The debug overlay root should have one child");

        *writer
            .get_text(*fps_span, 2)
            .expect("The state text span should be present.") = format!(
            "\nCurrent level:\n  {}\nCurrent scene: {}\n# of entities: {}",
            level
                .gltf
                .path()
                .expect("Gltf should be a strong handle and have a path."),
            scene.0,
            entities.iter().len()
        );
    }
}
