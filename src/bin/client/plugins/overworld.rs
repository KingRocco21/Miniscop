use crate::states::AppState;
use bevy::prelude::*;
use bevy_sprite3d::{Sprite3dBuilder, Sprite3dParams};
use bincode::{decode_from_slice, encode_to_vec};
use miniscop::networking::{Packet, PACKET_CONFIG};
use quinn::{rustls, ClientConfig, Endpoint, VarInt};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::lookup_host;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<OverworldState>()
            .add_systems(
                OnEnter(AppState::Overworld),
                (setup_overworld, setup_async_runtime),
            )
            .add_systems(
                FixedUpdate,
                advance_physics.run_if(in_state(OverworldState::InGame)),
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
            .add_systems(OnExit(AppState::Overworld), stop_async_runtime);
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
struct SpriteToBeSpawned(Handle<Image>);
/// This resource keeps the async server connection alive. When it is removed, the client will time out from the server.
// Todo: Add a better way of disconnecting
#[derive(Resource)]
struct ServerConnection {
    runtime: Runtime,
    connection_handle: JoinHandle<()>,
    packet_sender: Sender<Packet>,
}

// Components
#[derive(Component)]
struct Player;

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
    commands.insert_resource(SpriteToBeSpawned(
        asset_server.load("overworld/2d/sprites/guardian.png"),
    ));
}

fn setup_async_runtime(mut commands: Commands) {
    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
    let (tx, rx) = mpsc::channel::<Packet>(16);
    // Connect to server
    let connection_handle = runtime.spawn(async move {
        if let Err(e) = connect_to_server(rx).await {
            error!("Connection error: {:?}", e);
        }
    });

    commands.insert_resource(ServerConnection {
        runtime,
        connection_handle,
        packet_sender: tx,
    });
}

fn finish_loading(
    mut commands: Commands,
    sprite_to_be_spawned: Res<SpriteToBeSpawned>,
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut sprite3d_params: Sprite3dParams,
    mut next_state: ResMut<NextState<OverworldState>>,
) {
    for event in asset_events.read() {
        if event.is_loaded_with_dependencies(sprite_to_be_spawned.0.id()) {
            commands.spawn((
                StateScoped(AppState::Overworld),
                Sprite3dBuilder {
                    image: sprite_to_be_spawned.0.clone(),
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
            commands.remove_resource::<SpriteToBeSpawned>();
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

        // Need to normalize and scale because otherwise
        // diagonal movement would be faster than horizontal or vertical movement.
        // This effectively averages the accumulated input.
        velocity.0 = input.normalize_or_zero() * 4.0;
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

fn stop_async_runtime(mut commands: Commands) {
    commands.remove_resource::<ServerConnection>();
}

// Non-system functions
#[tracing::instrument()]
async fn connect_to_server(mut bevy_receiver: Receiver<Packet>) -> anyhow::Result<()> {
    let endpoint = Endpoint::client(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)))?;

    const URL: &str = "miniscop.twilightparadox.com";
    let server_address = lookup_host((URL, 4433))
        .await?
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not resolve the server's IP address"))?;
    info!("Connecting to {}", server_address);

    // Rustls needs to get the computer's crypto provider first, or else Quinn will panic.
    // https://github.com/quinn-rs/quinn/issues/2275
    rustls::client::ClientConfig::builder();

    let connection = endpoint
        .connect_with(ClientConfig::with_platform_verifier(), server_address, URL)
        .map_err(|e| anyhow::anyhow!("Connection configuration error: {:?}", e))?
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to server: {:?}", e))?;
    info!("Connected to {}", server_address);

    let mut recv = connection.accept_uni().await?;
    let packet = recv.read_to_end(64).await?;
    let (packet, len): (Packet, usize) = decode_from_slice(packet.as_slice(), PACKET_CONFIG)?;

    info!("Received {:?} from {:?} bytes", packet, len);

    connection.close(
        VarInt::from_u32(0),
        encode_to_vec("Client says goodbye!", PACKET_CONFIG)?.as_slice(),
    );
    endpoint.wait_idle().await;

    Ok(())
}
