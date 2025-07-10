mod animation;
mod multiplayer;
mod physics;

use crate::AppState;
use avian3d::prelude::{
    Collider, ColliderConstructor, ColliderConstructorHierarchy, Dominance, LockedAxes,
    PhysicsDebugPlugin, RigidBody,
};
use avian3d::PhysicsPlugins;
use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::{
    default, in_state, App, AppExtStates, AssetServer, Assets, AudioPlayer, AudioSource,
    Camera, Camera3d, ClearColorConfig, Color, Commands, Component, Condition,
    FixedLast, FixedUpdate, GltfAssetLabel, Handle, Image, IntoScheduleConfigs, NextState,
    OnEnter, PlaybackSettings, Plugin, Res, ResMut, Resource, Scene, SceneRoot, Single, StateScoped,
    StateSet, SubStates, TextureAtlas, TextureAtlasLayout, Timer, TimerMode, Transform, UVec2, Update,
    Vec3, With, Without,
};
use bevy_sprite3d::{Sprite3dBuilder, Sprite3dParams};
use bevy_tnua::prelude::{TnuaController, TnuaControllerPlugin};
use bevy_tnua::TnuaUserControlsSystemSet;
use bevy_tnua_avian3d::{TnuaAvian3dPlugin, TnuaAvian3dSensorShape};
use multiplayer::MultiplayerState;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PhysicsPlugins::default(),
            PhysicsDebugPlugin::default(),
            TnuaControllerPlugin::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
        ))
        .add_sub_state::<OverworldState>()
        .init_state::<MultiplayerState>()
        .add_event::<multiplayer::OtherPlayerMoved>()
        .add_event::<multiplayer::OtherPlayerDisconnected>()
        .add_systems(
            OnEnter(AppState::Overworld),
            (setup_overworld, multiplayer::setup_client_runtime),
        )
        .add_systems(
            Update,
            finish_loading.run_if(in_state(OverworldState::LoadingScreen)),
        )
        .add_systems(
            FixedUpdate,
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
                physics::apply_controls.in_set(TnuaUserControlsSystemSet),
                animation::animate_sprites,
            )
                .chain()
                .run_if(in_state(OverworldState::InGame)),
        )
        .add_systems(
            FixedLast,
            multiplayer::send_current_position.run_if(in_state(MultiplayerState::Online)),
        )
        .add_systems(
            Update,
            follow_player_with_camera.run_if(in_state(OverworldState::InGame)),
        )
        .add_systems(
            Update,
            multiplayer::stop_client_runtime_on_window_close
                .run_if(in_state(MultiplayerState::Online)),
        );
    }
}

// Constants
/// Note: Based on current guardian sprite
const SPRITE_PIXELS_PER_METER: f32 = 33.0;
const STARTING_TRANSLATION: Vec3 = Vec3::new(0.0, 0.5, 0.0);

// Sub-States
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(AppState = AppState::Overworld)]
#[states(scoped_entities)]
enum OverworldState {
    #[default]
    LoadingScreen,
    InGame,
}

// Resources
#[derive(Resource)]
struct OverworldAssetCollection {
    level: Handle<Scene>,
    sprites: OverworldSprites,
    sound_effects: OverworldSoundEffects,
    songs: OverworldSongs,
}
struct OverworldSprites {
    guardian_image: Handle<Image>,
    other_player_image: Handle<Image>,
    sprite_layout: Handle<TextureAtlasLayout>,
}
struct OverworldSoundEffects {
    walking_1: Handle<AudioSource>,
    walking_2: Handle<AudioSource>,
}
struct OverworldSongs {
    gift_plane: Handle<AudioSource>,
}

impl OverworldAssetCollection {
    fn all_assets_are_loaded(&self, asset_server: &Res<AssetServer>) -> bool {
        asset_server
            .get_load_state(self.level.id())
            .is_some_and(|state| state.is_loaded())
            && asset_server
                .get_load_state(self.sprites.guardian_image.id())
                .is_some_and(|state| state.is_loaded())
            && asset_server
                .get_load_state(self.sprites.other_player_image.id())
                .is_some_and(|state| state.is_loaded())
            && asset_server
                .get_load_state(self.sound_effects.walking_1.id())
                .is_some_and(|state| state.is_loaded())
            && asset_server
                .get_load_state(self.sound_effects.walking_2.id())
                .is_some_and(|state| state.is_loaded())
            && asset_server
                .get_load_state(self.songs.gift_plane.id())
                .is_some_and(|state| state.is_loaded())
    }
}

// Components
#[derive(Component)]
struct Player;

// Systems
fn setup_overworld(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Start loading assets
    commands.insert_resource(OverworldAssetCollection {
        level: asset_server
            .load(GltfAssetLabel::Scene(0).from_asset("overworld/3d/Gift_Plane.glb")),
        sprites: OverworldSprites {
            guardian_image: asset_server.load("overworld/2d/guardian.png"),
            other_player_image: asset_server.load("overworld/2d/other_player.png"),
            sprite_layout: texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
                UVec2::splat(64),
                5,
                5,
                None,
                None,
            )),
        },
        sound_effects: OverworldSoundEffects {
            walking_1: asset_server.load("overworld/sounds/walking_1.ogg"),
            walking_2: asset_server.load("overworld/sounds/walking_2.ogg"),
        },
        songs: OverworldSongs {
            gift_plane: asset_server.load("overworld/sounds/gift_plane.ogg"),
        },
    });
}

fn finish_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    assets: Res<OverworldAssetCollection>,
    mut sprite3d_params: Sprite3dParams,
    mut next_state: ResMut<NextState<OverworldState>>,
) {
    if assets.all_assets_are_loaded(&asset_server) {
        // Spawn level
        commands.spawn((
            StateScoped(AppState::Overworld),
            SceneRoot(assets.level.clone()),
            Transform::default(),
            RigidBody::Static,
            ColliderConstructorHierarchy::new(None).with_constructor_for_name(
                "Hitbox Mesh",
                ColliderConstructor::ConvexDecompositionFromMesh,
            ),
        ));
        // Spawn player
        commands.spawn((
            StateScoped(AppState::Overworld),
            Player,
            Sprite3dBuilder {
                image: assets.sprites.guardian_image.clone(),
                pixels_per_metre: SPRITE_PIXELS_PER_METER,
                double_sided: false,
                unlit: true,
                ..default()
            }
            .bundle_with_atlas(
                &mut sprite3d_params,
                TextureAtlas {
                    layout: assets.sprites.sprite_layout.clone(),
                    index: 0,
                },
            ),
            Transform::from_translation(STARTING_TRANSLATION),
            animation::AnimationTimer(Timer::from_seconds(0.15, TimerMode::Repeating)),
            animation::AnimationDirection(Vec3::ZERO),
            RigidBody::Dynamic,
            Collider::cuboid(1.0, 1.0, 1.0),
            TnuaController::default(),
            TnuaAvian3dSensorShape(Collider::cuboid(1.0, 0.0, 1.0)),
            LockedAxes::ROTATION_LOCKED,
            Dominance(1),
        ));

        // Spawn music
        commands.spawn((
            StateScoped(AppState::Overworld),
            AudioPlayer::new(assets.songs.gift_plane.clone()),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::Linear(0.5),
                ..default()
            },
        ));

        // Spawn camera
        commands.spawn((
            StateScoped(AppState::Overworld),
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::WHITE),
                ..default()
            },
            Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));

        next_state.set(OverworldState::InGame);
    }
}

fn follow_player_with_camera(
    player_transform: Single<&Transform, With<Player>>,
    mut camera_transform: Single<&mut Transform, (With<Camera3d>, Without<Player>)>,
) {
    camera_transform.translation.x = camera_transform.translation.x.clamp(
        player_transform.translation.x - 2.0,
        player_transform.translation.x + 2.0,
    );
}
