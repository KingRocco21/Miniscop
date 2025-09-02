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
            LoadingState::new(OverworldState::LoadingOverworld)
                .load_collection::<OverworldAssets>()
                .continue_to_state(OverworldState::LoadingLevel),
        )
        .add_systems(OnExit(OverworldState::LoadingOverworld), setup_overworld)
        .add_loading_state(
            LoadingState::new(OverworldState::LoadingLevel)
                .load_collection::<LevelAssets>()
                .continue_to_state(OverworldState::LoadingScene),
        )
        .add_systems(OnExit(OverworldState::LoadingLevel), setup_level)
        .add_systems(OnEnter(OverworldState::LoadingScene), setup_scene)
        .add_observer(on_add_interaction)
        .add_observer(on_add_animation_prompt)
        .add_observer(on_add_animated_rotation);
    }
}

// Constants
/// Note: Based on current guardian sprite
pub const SPRITE_PIXELS_PER_METER: f32 = 33.0;
pub const STARTING_PLAYER_TRANSLATION: Vec3 = Vec3::new(-2.0, input::FLOAT_HEIGHT, 0.0);
pub const STARTING_CAMERA_TRANSLATION: Vec3 = Vec3::new(0.0, 4.0, 9.0);

#[derive(AssetCollection, Resource)]
pub struct OverworldAssets {
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
pub struct LevelAssets {
    #[asset(key = "gltf")]
    pub gltf: Handle<Gltf>,
    #[asset(key = "music")]
    pub music: Handle<AudioSource>,
}

/// Check that the current level has the desired scene number BEFORE changing the scene.
#[derive(Resource)]
pub struct CurrentScene(pub usize);

// Systems
pub fn setup_overworld(mut commands: Commands, sprites: Res<OverworldAssets>) {
    // Spawn floor
    commands.spawn((
        StateScoped(AppState::Overworld),
        Transform::from_xyz(0.0, -0.5, 0.0),
        RigidBody::Static,
        Collider::cuboid(1000.0, 1.0, 1000.0),
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

fn setup_level(mut commands: Commands, _level: Res<LevelAssets>) {
    // Set room number
    commands.insert_resource(CurrentScene(0));

    // Spawn music
    // commands.spawn((
    //     StateScoped(AppState::Overworld),
    //     AudioPlayer::new(level.music.clone()),
    //     PlaybackSettings {
    //         mode: PlaybackMode::Loop,
    //         volume: Volume::Linear(0.5),
    //         ..default()
    //     },
    // ));
}

fn setup_scene(
    mut commands: Commands,
    level: Res<LevelAssets>,
    gltf: Res<Assets<Gltf>>,
    current_scene: Res<CurrentScene>,
    mut next_state: ResMut<NextState<OverworldState>>,
) {
    // Spawn room
    let scene = &gltf
        .get(level.gltf.id())
        .expect("The gltf should be loaded.")
        .scenes[current_scene.0];
    commands.spawn((
        StateScoped(OverworldState::InScene),
        SceneRoot(scene.clone()),
        Transform::default(),
    ));
    next_state.set(OverworldState::InScene);
    info!("Scene is done loading");
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
    commands
        .entity(trigger.target())
        .insert(animation::InitialTransform(transform.clone()));
}
