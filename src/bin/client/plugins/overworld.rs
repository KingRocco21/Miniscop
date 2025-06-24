use crate::networking::{setup_client_runtime, stop_client_runtime, ServerConnection};
use crate::states::AppState;
use bevy::prelude::*;
use bevy_sprite3d::{Sprite3dBuilder, Sprite3dParams};
use miniscop::networking::Packet;
use tokio::sync::mpsc::error::TrySendError;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<OverworldState>()
            .add_event::<OtherPlayerMoved>()
            .add_systems(
                OnEnter(AppState::Overworld),
                (setup_overworld, setup_client_runtime),
            )
            .add_systems(
                FixedUpdate,
                (
                    (
                        // These must be run in this order because each one is dependent on the next.
                        read_packets,
                        on_other_player_moved,
                        advance_physics.run_if(in_state(OverworldState::InGame)),
                    )
                        .chain(),
                    send_current_position,
                )
                    .run_if(in_state(AppState::Overworld)),
            )
            .add_systems(
                RunFixedMainLoop,
                (
                    handle_input.in_set(RunFixedMainLoopSystem::BeforeFixedMainLoop),
                    interpolate_rendered_transform
                        .in_set(RunFixedMainLoopSystem::AfterFixedMainLoop),
                )
                    .run_if(in_state(OverworldState::InGame)),
            )
            .add_systems(
                Update,
                finish_loading.run_if(in_state(OverworldState::Loading)),
            )
            .add_systems(
                Update,
                follow_player_with_camera.run_if(in_state(OverworldState::InGame)),
            )
            .add_systems(OnExit(AppState::Overworld), stop_client_runtime);
    }
}

// Constants
/// Note: Based on current guardian sprite
const SPRITE_PIXELS_PER_METER: f32 = 132.0;
/// Note: Only applicable to gift plane
const STARTING_TRANSLATION: Vec3 = Vec3::new(0.0, 180.0 / SPRITE_PIXELS_PER_METER * 0.5, 0.0);

// Overworld Sub-States
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(AppState = AppState::Overworld)]
#[states(scoped_entities)]
enum OverworldState {
    #[default]
    Loading,
    InGame,
}

// Resources
#[derive(Resource)]
struct GuardianSprite(Handle<Image>);

// Components
#[derive(Component)]
struct Player;
#[derive(Component)]
struct OtherPlayer {
    id: u32,
}

// Physics Components
// https://github.com/bevyengine/bevy/blob/latest/examples/movement/physics_in_fixed_timestep.rs
/// A vector representing the player's input, accumulated over all frames that ran
/// since the last time the physics simulation was advanced.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct AccumulatedInput(Vec3);

/// A vector representing the player's velocity in the physics simulation.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct Velocity(Vec3);

/// The actual position of the player in the physics simulation.
/// This is separate from the `Transform`, which is merely a visual representation.
///
/// If you want to make sure that this component is always initialized
/// with the same value as the `Transform`'s translation, you can
/// use a [component lifecycle hook](https://docs.rs/bevy/0.14.0/bevy/ecs/component/struct.ComponentHooks.html)
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct PhysicalTranslation(Vec3);

/// The value [`PhysicalTranslation`] had in the last fixed timestep.
/// Used for interpolation in the `interpolate_rendered_transform` system.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct PreviousPhysicalTranslation(Vec3);

// Systems
fn setup_overworld(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Main 3D camera
    commands.spawn((
        StateScoped(AppState::Overworld),
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::WHITE),
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        AmbientLight {
            brightness: 1000.0,
            ..default()
        },
    ));
    // Spawn blender scene
    commands.spawn((
        StateScoped(AppState::Overworld),
        SceneRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("overworld/3d/Gift_Plane.glb")),
        ),
        Transform::default(),
    ));
    // Start loading guardian
    commands.insert_resource(GuardianSprite(
        asset_server.load("overworld/2d/sprites/guardian.png"),
    ));
}

fn finish_loading(
    mut commands: Commands,
    guardian_sprite: Res<GuardianSprite>,
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut sprite3d_params: Sprite3dParams,
    mut next_state: ResMut<NextState<OverworldState>>,
) {
    for event in asset_events.read() {
        if event.is_loaded_with_dependencies(guardian_sprite.0.id()) {
            commands.spawn((
                StateScoped(AppState::Overworld),
                Sprite3dBuilder {
                    image: guardian_sprite.0.clone(),
                    pixels_per_metre: SPRITE_PIXELS_PER_METER,
                    double_sided: false,
                    unlit: true,
                    ..default()
                }
                .bundle(&mut sprite3d_params),
                Transform::from_translation(STARTING_TRANSLATION),
                AccumulatedInput::default(),
                Velocity::default(),
                PhysicalTranslation(STARTING_TRANSLATION),
                PreviousPhysicalTranslation(STARTING_TRANSLATION),
                Player,
            ));
            next_state.set(OverworldState::InGame);
        }
    }
}

/// Handle keyboard input and accumulate it in the `AccumulatedInput` component.
fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut AccumulatedInput, &mut Velocity)>,
) {
    for (mut input, mut velocity) in query.iter_mut() {
        if keyboard_input.pressed(KeyCode::KeyW) {
            input.z -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            input.z += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            input.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            input.x += 1.0;
        }

        // If you want to normalize the input, do input.normalize_or_zero() instead of clamping.
        velocity.0 = input.clamp(Vec3::NEG_ONE, Vec3::ONE) * 4.0;
    }
}

/// Advance the physics simulation by one fixed timestep. This may run zero or multiple times per frame.
///
/// Note that since this runs in `FixedUpdate`, `Res<Time>` would be `Res<Time<Fixed>>` automatically.
/// We are being explicit here for clarity.
fn advance_physics(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut PhysicalTranslation,
        &mut PreviousPhysicalTranslation,
        &mut AccumulatedInput,
        &Velocity,
    )>,
) {
    for (
        mut current_physical_translation,
        mut previous_physical_translation,
        mut input,
        velocity,
    ) in query.iter_mut()
    {
        previous_physical_translation.0 = current_physical_translation.0;
        current_physical_translation.0 += velocity.0 * fixed_time.delta_secs();

        // Reset the input accumulator, as we are currently consuming all input that happened since the last fixed timestep.
        input.0 = Vec3::ZERO;
    }
}

fn interpolate_rendered_transform(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut Transform,
        &PhysicalTranslation,
        &PreviousPhysicalTranslation,
    )>,
) {
    for (mut transform, current_physical_translation, previous_physical_translation) in
        query.iter_mut()
    {
        let previous = previous_physical_translation.0;
        let current = current_physical_translation.0;
        // The overstep fraction is a value between 0 and 1 that tells us how far we are between two fixed timesteps.
        let alpha = fixed_time.overstep_fraction();

        let rendered_translation = previous.lerp(current, alpha);
        transform.translation = rendered_translation;
    }
}

fn follow_player_with_camera(
    player_transform: Single<&Transform, With<Player>>,
    mut camera_transform: Single<&mut Transform, (With<Camera3d>, Without<Player>)>,
) {
    // Get the player's x distance to the camera.
    let x_dist = player_transform.translation.x - camera_transform.translation.x;
    if x_dist > 2.0 {
        camera_transform.translation.x = player_transform.translation.x - 2.0;
    } else if x_dist < -2.0 {
        camera_transform.translation.x = player_transform.translation.x + 2.0;
    }
}

// Events
#[derive(Event)]
struct OtherPlayerMoved {
    id: u32,
    x: f32,
    y: f32,
    z: f32,
}

/// This system reads incoming packets, and fires a matching event for each one.
#[tracing::instrument(skip(connection, player_moved))]
fn read_packets(
    mut connection: ResMut<ServerConnection>,
    mut player_moved: EventWriter<OtherPlayerMoved>,
) {
    // let time = Instant::now();
    while let Ok(packet) = connection.from_server.try_recv() {
        match packet {
            Packet::PlayerPosition { id, x, y, z } => {
                player_moved.write(OtherPlayerMoved { id, x, y, z });
            }
        }
    }
    // info!("Took {:?}", time.elapsed());
}

fn send_current_position(
    connection: Res<ServerConnection>,
    position: Single<&PhysicalTranslation>,
) {
    // Only send packets if connected to server
    if connection.handle.is_finished() {
        let packet = Packet::PlayerPosition {
            id: 0,
            x: position.x,
            y: position.y,
            z: position.z,
        };

        if let Err(TrySendError::Full(_)) = connection.to_client.try_send(packet) {
            panic!(
                "Packet channel to async should never be full.\nIf you see this, please report this error so the dev can consider increasing channel size."
            )
        }
    }
}

/// This system updates the transforms of other players, and spawns the player if they don't exist yet.
fn on_other_player_moved(
    mut commands: Commands,
    guardian_sprite: Res<GuardianSprite>,
    mut sprite3d_params: Sprite3dParams,
    mut player_moved: EventReader<OtherPlayerMoved>,
    mut query: Query<(&OtherPlayer, &mut Transform)>,
) {
    for movement in player_moved.read() {
        let mut found_player = false;
        for (other_player, mut transform) in query.iter_mut() {
            if other_player.id == movement.id {
                // Todo: Add PhysicalTranslation for smooth movement onscreen. Careful though, advance_physics doesn't run when the game is paused!
                let translation = Vec3::new(movement.x, movement.y, movement.z);
                transform.translation = translation;
                found_player = true;
            }
        }
        if !found_player {
            commands.spawn((
                StateScoped(AppState::Overworld),
                OtherPlayer { id: movement.id },
                Sprite3dBuilder {
                    image: guardian_sprite.0.clone(),
                    pixels_per_metre: SPRITE_PIXELS_PER_METER,
                    double_sided: false,
                    unlit: true,
                    ..default()
                }
                .bundle(&mut sprite3d_params),
                Transform::from_xyz(movement.x, movement.y, movement.z),
            ));
        }
    }
}
