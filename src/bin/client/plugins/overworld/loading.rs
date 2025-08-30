use crate::plugins::overworld::{animation, input, OverworldState, Player};
use crate::AppState;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_asset_loader::loading_state::{LoadingState, LoadingStateAppExt};
use bevy_asset_loader::prelude::{AssetCollection, ConfigureLoadingState};
use bevy_sprite3d::Sprite3d;
use bevy_tnua::controller::TnuaController;
use bevy_tnua_avian3d::TnuaAvian3dSensorShape;
use std::f32::consts::PI;

pub struct LoadingPlugin;
impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_loading_state(
            LoadingState::new(OverworldState::Loading)
                .load_collection::<LevelAssets>()
                .load_collection::<SpriteAssets>()
                .load_collection::<SoundAssets>()
                .load_collection::<SongAssets>()
                .continue_to_state(OverworldState::InGame),
        )
        .add_observer(on_add_interaction)
        .add_observer(on_add_animation_prompt)
        .add_observer(on_add_animated_rotation)
        .add_systems(OnEnter(OverworldState::InGame), setup_overworld);
    }
}

// Constants
/// Note: Based on current guardian sprite
pub const SPRITE_PIXELS_PER_METER: f32 = 33.0;
pub const STARTING_PLAYER_TRANSLATION: Vec3 = Vec3::new(-2.0, input::FLOAT_HEIGHT, 0.0);
pub const STARTING_CAMERA_TRANSLATION: Vec3 = Vec3::new(0.0, 5.0, 10.0);

// Resources
#[derive(AssetCollection, Resource)]
pub struct LevelAssets {
    #[asset(path = "overworld/3d/Gift_Plane.glb#Scene0")]
    pub gift_plane: Handle<Scene>,
}
#[derive(AssetCollection, Resource)]
pub struct SpriteAssets {
    #[asset(path = "overworld/2d/guardian.png")]
    guardian_image: Handle<Image>,
    #[asset(path = "overworld/2d/other_player.png")]
    pub other_player_image: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 64, tile_size_y = 64, columns = 5, rows = 5))]
    pub sprite_layout: Handle<TextureAtlasLayout>,
    #[asset(path = "overworld/2d/text_box.png")]
    pub text_box_image: Handle<Image>,
    #[asset(path = "overworld/2d/text_box_arrow.png")]
    pub text_box_arrow_image: Handle<Image>,
}
#[derive(AssetCollection, Resource)]
pub struct SoundAssets {
    #[asset(path = "overworld/sounds/walking_1.ogg")]
    pub walking_1: Handle<AudioSource>,
    #[asset(path = "overworld/sounds/walking_2.ogg")]
    pub walking_2: Handle<AudioSource>,
    #[asset(path = "overworld/sounds/approaching_interactable.ogg")]
    pub approaching_interactable: Handle<AudioSource>,
    #[asset(path = "overworld/sounds/dialogue_start.ogg")]
    pub dialogue_start: Handle<AudioSource>,
    #[asset(path = "overworld/sounds/dialogue.ogg")]
    pub dialogue: Handle<AudioSource>,
    #[asset(path = "overworld/sounds/dialogue_end.ogg")]
    pub dialogue_end: Handle<AudioSource>,
}
#[derive(AssetCollection, Resource)]
pub struct SongAssets {
    #[asset(path = "overworld/sounds/gift_plane.ogg")]
    pub gift_plane: Handle<AudioSource>,
}

// Systems
pub fn setup_overworld(
    mut commands: Commands,
    levels: Res<LevelAssets>,
    sprites: Res<SpriteAssets>,
) {
    // Spawn floor
    commands.spawn((
        StateScoped(AppState::Overworld),
        Transform::from_xyz(0.0, -0.5, 0.0),
        RigidBody::Static,
        Collider::cuboid(1000.0, 1.0, 1000.0),
    ));
    // Spawn level
    commands.spawn((
        StateScoped(AppState::Overworld),
        SceneRoot(levels.gift_plane.clone()),
        Transform::default(),
    ));

    // Spawn player
    commands.spawn((
        StateScoped(AppState::Overworld),
        Player,
        Transform::from_translation(STARTING_PLAYER_TRANSLATION),
        // Physics
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 1.0, 1.0),
        LockedAxes::ROTATION_LOCKED,
        Dominance(1),
        // Character Controller
        TnuaController::default(),
        TnuaAvian3dSensorShape(Collider::cuboid(0.8, 0.0, 0.8)),
        // Fix for https://bevy.org/learn/errors/b0004/
        Visibility::default(),
        children![(
            // Sprite (must be rotated separately from the collider)
            Sprite {
                image: sprites.guardian_image.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: sprites.sprite_layout.clone(),
                    index: 0,
                }),
                ..default()
            },
            Sprite3d {
                pixels_per_metre: SPRITE_PIXELS_PER_METER,
                unlit: true,
                double_sided: false,
                ..default()
            },
            // Animation
            animation::AnimationTimer(Timer::from_seconds(0.15, TimerMode::Repeating)),
        )],
    ));

    // Spawn music
    // commands.spawn((
    //     StateScoped(AppState::Overworld),
    //     AudioPlayer::new(songs.gift_plane.clone()),
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
}

/// We must observe when Skein spawns the interaction sensor so we can add collision observers.
pub fn on_add_interaction(
    trigger: Trigger<OnAdd, input::interaction::OverworldInteraction>,
    mut commands: Commands,
) {
    commands
        .entity(trigger.target())
        .observe(input::interaction::when_approaching_interactable)
        .observe(input::interaction::when_leaving_interactable);
}

/// We must observe when Skein spawns the interaction prompt so we can hide it by default.
pub fn on_add_animation_prompt(
    trigger: Trigger<OnAdd, animation::AnimatedInteractionPromptState>,
    mut transform_query: Query<&mut Transform>,
    mut commands: Commands,
) {
    let mut transform = transform_query
        .get_mut(trigger.target())
        .expect("Transform not present, please report to dev!");
    transform.scale = Vec3::ZERO;
    commands
        .entity(trigger.target())
        .insert(animation::InitialTransform(transform.clone()));
}

pub fn on_add_animated_rotation(
    trigger: Trigger<OnAdd, animation::AnimatedRotation>,
    transform_query: Query<&Transform>,
    mut commands: Commands,
) {
    let transform = transform_query
        .get(trigger.target())
        .expect("Transform not present, please report to dev!");
    info!("{}", transform.scale);
    commands
        .entity(trigger.target())
        .insert(animation::InitialTransform(transform.clone()));
}
