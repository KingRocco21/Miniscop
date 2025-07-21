mod animation;
mod input;
mod multiplayer;

use crate::AppState;
use avian3d::prelude::{
    Collider, ColliderConstructor, CollisionEventsEnabled, Dominance, LockedAxes, OnCollisionEnd,
    OnCollisionStart, PhysicsDebugPlugin, RigidBody, Sensor,
};
use avian3d::PhysicsPlugins;
use bevy::audio::{AudioPlayer, PlaybackMode, PlaybackSettings};
use bevy::prelude::{
    default, in_state, AmbientLight, App, AppExtStates, AssetServer, Assets, AudioSource, Camera,
    Camera3d, Children, ClearColorConfig, Color, Commands, Component, Condition,
    DirectionalLight, FixedFirst, FixedLast, FixedPreUpdate, GltfAssetLabel, Handle, Image, IntoScheduleConfigs,
    Name, NextState, OnEnter, Plugin, Quat, Query, Res, ResMut, Resource, Scene, SceneRoot,
    Single, StateScoped, StateSet, SubStates, TextureAtlas, TextureAtlasLayout, Timer, TimerMode,
    Transform, Trigger, UVec2, Update, Vec3, With, Without,
};
use bevy::scene::SceneInstanceReady;
use bevy_sprite3d::{Sprite3dBuilder, Sprite3dParams};
use bevy_tnua::prelude::{TnuaController, TnuaControllerPlugin};
use bevy_tnua::TnuaUserControlsSystemSet;
use bevy_tnua_avian3d::{TnuaAvian3dPlugin, TnuaAvian3dSensorShape};
use leafwing_input_manager::prelude::InputManagerPlugin;
use multiplayer::MultiplayerState;
use std::f32::consts::PI;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PhysicsPlugins::default(),
            PhysicsDebugPlugin::default(),
            TnuaControllerPlugin::new(FixedPreUpdate),
            TnuaAvian3dPlugin::new(FixedPreUpdate),
            InputManagerPlugin::<input::PlayerAction>::default(),
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
                input::respawn,
                input::walk.in_set(TnuaUserControlsSystemSet),
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
                animation::animate_walk_cycles,
                animation::animate_interaction_prompts,
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

// Constants
/// Note: Based on current guardian sprite
const SPRITE_PIXELS_PER_METER: f32 = 33.0;
const STARTING_PLAYER_TRANSLATION: Vec3 = Vec3::new(-2.0, input::FLOAT_HEIGHT, 0.0);
const STARTING_CAMERA_TRANSLATION: Vec3 = Vec3::new(0.0, 4.0, 8.0);

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
    approaching_interactable: Handle<AudioSource>,
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
            approaching_interactable: asset_server
                .load("overworld/sounds/approaching_interactable.ogg"),
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
        // Spawn floor
        commands.spawn((
            StateScoped(AppState::Overworld),
            Transform::from_xyz(0.0, -0.5, 0.0),
            RigidBody::Static,
            Collider::cuboid(1000.0, 1.0, 1000.0),
        ));
        // Spawn level
        commands
            .spawn((
                StateScoped(AppState::Overworld),
                SceneRoot(assets.level.clone()),
                Transform::default(),
            ))
            .observe(on_level_spawn);

        // Spawn player
        commands.spawn((
            StateScoped(AppState::Overworld),
            Player,
            Transform::from_translation(STARTING_PLAYER_TRANSLATION),
            // Sprite
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
            // Animation
            animation::WalkCycleTimer(Timer::from_seconds(0.15, TimerMode::Repeating)),
            // Physics
            RigidBody::Dynamic,
            Collider::cuboid(1.0, 1.0, 0.2),
            LockedAxes::ROTATION_LOCKED,
            Dominance(1),
            // Character Controller
            TnuaController::default(),
            TnuaAvian3dSensorShape(Collider::cuboid(0.9, 0.0, 0.1)),
            // Input
            input::PlayerAction::default_input_map(),
        ));

        // Spawn music
        // commands.spawn((
        //     StateScoped(AppState::Overworld),
        //     AudioPlayer::new(assets.songs.gift_plane.clone()),
        //     PlaybackSettings {
        //         mode: PlaybackMode::Loop,
        //         volume: Volume::Linear(0.5),
        //         ..default()
        //     },
        // ));

        // Spawn camera
        commands.spawn((
            StateScoped(AppState::Overworld),
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::WHITE),
                ..default()
            },
            Transform::from_translation(STARTING_CAMERA_TRANSLATION)
                .looking_at(Vec3::new(STARTING_CAMERA_TRANSLATION.x, 0.0, 0.0), Vec3::Y),
            AmbientLight {
                brightness: 1000.0,
                ..default()
            },
        ));

        commands.spawn((
            StateScoped(AppState::Overworld),
            DirectionalLight::default(),
            Transform::from_rotation(Quat::from_rotation_y(PI / 4.0)),
        ));

        next_state.set(OverworldState::InGame);
    }
}

/// On level spawn, add the relevant components to each blender mesh.
fn on_level_spawn(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    names: Query<&Name>,
    mut transforms: Query<&mut Transform>,
) {
    for child in children.iter_descendants(trigger.target()) {
        if let Ok(name) = names.get(child) {
            if name.contains("Hitbox Mesh") {
                commands.entity(child).insert((
                    RigidBody::Static,
                    ColliderConstructor::ConvexDecompositionFromMesh,
                ));
            } else if name.as_str() == "Animated Interaction Prompt" {
                let mut initial_transform = transforms
                    .get_mut(child)
                    .expect("The interaction prompt should have a transform already");
                initial_transform.scale = Vec3::ZERO;

                // This needs to be a separate entity because the sensor should not move when the prompt moves.
                commands
                    .spawn((
                        StateScoped(AppState::Overworld),
                        Transform::from_translation(initial_transform.translation),
                        RigidBody::Static,
                        Collider::cuboid(3.0, 3.0, 3.0),
                        Sensor,
                        CollisionEventsEnabled,
                    ))
                    .observe(when_approaching_interactable)
                    .observe(when_leaving_interactable);

                commands.entity(child).insert((
                    // Make the interaction prompt invisible until it is approached.
                    animation::AnimatedInteractionPromptState::Hidden,
                    // Clone the initial translation and rotation so the transform can be modified freely.
                    animation::InitialTransform(initial_transform.clone()),
                ));
            }
        }
    }
}

fn when_approaching_interactable(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    assets: Res<OverworldAssetCollection>,
    transform_query: Query<&Transform>,
    mut interaction_prompt_query: Query<(
        &animation::InitialTransform,
        &mut animation::AnimatedInteractionPromptState,
    )>,
) {
    // Play sound
    commands.spawn((
        StateScoped(AppState::Overworld),
        AudioPlayer::new(assets.sound_effects.approaching_interactable.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            ..default()
        },
    ));
    // Make the corresponding prompt appear
    let sensor_entity = trigger.target();
    let sensor_translation = transform_query
        .get(sensor_entity)
        .expect("The sensor entity should have a transform")
        .translation;
    for (initial_transform, mut prompt_state) in interaction_prompt_query.iter_mut() {
        if initial_transform.translation == sensor_translation {
            *prompt_state = animation::AnimatedInteractionPromptState::Growing;
        } else {
            *prompt_state = animation::AnimatedInteractionPromptState::Shrinking;
        }
    }
}

fn when_leaving_interactable(
    trigger: Trigger<OnCollisionEnd>,
    transform_query: Query<&Transform>,
    mut interaction_prompt_query: Query<(
        &animation::InitialTransform,
        &mut animation::AnimatedInteractionPromptState,
    )>,
) {
    let sensor_entity = trigger.target();
    let sensor_translation = transform_query
        .get(sensor_entity)
        .expect("The sensor entity should have a transform")
        .translation;
    for (initial_transform, mut prompt_state) in interaction_prompt_query.iter_mut() {
        if initial_transform.translation == sensor_translation {
            *prompt_state = animation::AnimatedInteractionPromptState::Shrinking;
        }
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
