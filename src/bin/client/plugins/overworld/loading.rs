use crate::plugins::overworld::input::interaction;
use crate::plugins::overworld::{animation, input, OverworldState, Player};
use crate::AppState;
use avian3d::prelude::*;
use bevy::app::AppExit;
use bevy::audio::{PlaybackMode, Volume};
use bevy::gltf::GltfMeshExtras;
use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;
use bevy_asset_loader::prelude::AssetCollection;
use bevy_sprite3d::Sprite3d;
use bevy_tnua::controller::TnuaController;
use bevy_tnua_avian3d::TnuaAvian3dSensorShape;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use tracing::error;

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
    commands
        .spawn((
            StateScoped(AppState::Overworld),
            SceneRoot(levels.gift_plane.clone()),
            Transform::default(),
        ))
        .observe(on_level_spawn);

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

// Blender custom properties
#[derive(Serialize, Deserialize, Debug)]
struct BlenderPhysicsProperties {
    rigid_body: RigidBody,
    collider_constructor: ColliderConstructor,
}

/// If you add "Interaction" as a custom property to an object, you MUST nest an "Animated Interaction Prompt" object inside of it as well.
#[derive(Serialize, Deserialize, Debug)]
struct BlenderInteractionProperties {
    interaction: BlenderInteraction,
    text: Option<String>,
}

/// You cannot add the data to these enum variants because Blender does not have support for it.
/// Instead, each BlenderInteraction expects its corresponding BlenderInteractionProperties value to be Some().
#[derive(Serialize, Deserialize, Debug)]
enum BlenderInteraction {
    Text,
}

/// On level spawn, add components to Blender meshes and spawn new entities for Blender objects based on their custom properties.
/// It would be nice to use Blenvy for this, but it's out of date.
fn on_level_spawn(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    gltf_extras: Query<&GltfExtras>,
    mut transforms: Query<&mut Transform>,
    names: Query<&Name>,
    mut app_exit: EventWriter<AppExit>,
    gltf_mesh_extras: Query<&GltfMeshExtras>,
) {
    for blender_object_or_mesh in children.iter_descendants(trigger.target()) {
        // Check objects for custom properties
        if let Ok(object_properties) = gltf_extras.get(blender_object_or_mesh) {
            if let Ok(interaction_properties) =
                serde_json::from_str::<BlenderInteractionProperties>(&object_properties.value)
            {
                // Regardless of the type of interactable, we first need to add a sensor around the object.
                // Here's how interactables work:
                // 1. Get the translation of the interactable
                let interactable_object = commands.entity(blender_object_or_mesh);
                let sensor_translation = transforms
                    .get(interactable_object.id())
                    .expect("The interactable object should have a transform already")
                    .translation;

                // 2. Attempt to find the interactable's interaction prompt
                let mut sensor_result: Option<Entity> = None;
                for nested_object in children.iter_descendants(interactable_object.id()) {
                    if let Ok(name) = names.get(nested_object)
                        && name.as_str() == "Animated Interaction Prompt"
                    {
                        // 3. Spawn a sensor that is linked to the interaction prompt.
                        // You can't add the sensor as a child to the "interactable_object" or else it will disappear for some reason,
                        // and you can't insert these components into the "interactable_object" or else its transform will change,
                        // so we must spawn it as a new entity.
                        let sensor = commands
                            .spawn((
                                StateScoped(AppState::Overworld),
                                Transform::from_translation(sensor_translation),
                                RigidBody::Static,
                                Collider::cuboid(3.0, 3.0, 3.0),
                                Sensor,
                                CollisionEventsEnabled,
                                interaction::InteractableWithPrompt(nested_object),
                            ))
                            .observe(interaction::when_approaching_interactable)
                            .observe(interaction::when_leaving_interactable)
                            .id();

                        // 4. Hide the interaction prompt by default
                        let mut prompt_transform = transforms
                            .get_mut(nested_object)
                            .expect("The interaction prompt should have a transform already");
                        prompt_transform.scale = Vec3::ZERO;

                        // 5. Add animation components to the interaction prompt so it can bob up and down.
                        commands.entity(nested_object).insert((
                            // Make the interaction prompt invisible until it is approached.
                            animation::AnimatedInteractionPromptState::Hidden,
                            // Clone the initial translation and rotation so the transform can be modified freely.
                            animation::InitialTransform(prompt_transform.clone()),
                        ));
                        // Only one sensor and interaction prompt is allowed per interactable.
                        // If multiple prompts are nested in the same object in Blender, only the first one will be used.
                        sensor_result = Some(sensor);
                        break;
                    }
                }
                // If a sensor was created, we can continue.
                match sensor_result {
                    None => {
                        error!(
                            "\"Animated Interaction Prompt\" was not nested in the interactable object {:?}. The level cannot be loaded properly.",
                            names.get(blender_object_or_mesh)
                        );
                        app_exit.write(AppExit::from_code(1));
                    }
                    Some(sensor) => match interaction_properties.interaction {
                        BlenderInteraction::Text => match interaction_properties.text {
                            None => {
                                error!(
                                    "A text interaction was added to the object {:?}, but no text property is present. The level cannot be loaded properly.",
                                    names.get(blender_object_or_mesh)
                                );
                                app_exit.write(AppExit::from_code(1));
                            }
                            Some(text) => {
                                commands
                                    .entity(sensor)
                                    .insert(interaction::OverworldInteraction::Text(text));
                            }
                        },
                    },
                }
            }
            // May add additional types of object properties in the future.
            else {
                error!(
                    "Found object properties that could not be deserialized into any known type: {:?}",
                    object_properties
                );
            }
        }
        // Check meshes for custom properties
        else if let Ok(mesh_properties) = gltf_mesh_extras.get(blender_object_or_mesh) {
            if let Ok(physics_properties) =
                serde_json::from_str::<BlenderPhysicsProperties>(&mesh_properties.value)
            {
                commands.entity(blender_object_or_mesh).insert((
                    physics_properties.rigid_body,
                    physics_properties.collider_constructor,
                ));
            }
            // May add additional types of mesh properties in the future.
            else {
                error!(
                    "Found mesh properties that could not be deserialized into any known type: {:?}",
                    mesh_properties
                );
            }
        }
    }
}
