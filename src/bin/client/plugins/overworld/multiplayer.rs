mod netcode;

use crate::plugins::overworld::{OverworldAssetCollection, SPRITE_PIXELS_PER_METER};
use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use bevy_rapier3d::prelude::Velocity;
use bevy_sprite3d::{Sprite3d, Sprite3dBuilder, Sprite3dParams};
use miniscop::networking::Packet;
use netcode::connect_to_server;
use quinn::{Connection, Endpoint};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

// States
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
#[states(scoped_entities)]
pub enum MultiplayerState {
    #[default]
    Offline,
    Connecting,
    Online,
}

// Resources
/// This resource keeps the async server connection alive.
///
/// This is guaranteed to exist when MultiplayerState is Connecting or Online.
#[derive(Resource)]
pub(crate) struct ServerConnection {
    runtime: Runtime,
    pub connection_handle:
        JoinHandle<anyhow::Result<(Endpoint, Connection, JoinHandle<()>, JoinHandle<()>)>>,
    pub to_client: Sender<Packet>,
    pub from_server: Receiver<Packet>,
}
// Todo: Add reconnecting support
impl ServerConnection {
    /// Try to gracefully disconnect from the server.
    ///
    /// You can force a disconnection by removing the ServerConnection resource.
    #[tracing::instrument(skip(self))]
    pub(crate) fn try_disconnect(&mut self) -> anyhow::Result<()> {
        self.to_client.try_send(Packet::ClientDisconnect(None))?;

        let connect_to_server_output = self.runtime.block_on(&mut self.connection_handle)?;
        match connect_to_server_output {
            Ok((_endpoint, connection, bevy_handle, server_handle)) => {
                self.runtime.block_on(bevy_handle)?;
                self.runtime.block_on(server_handle)?;
                self.runtime.block_on(connection.closed());
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Client cannot disconnect due to an error that was already reported."
                ));
            }
        }
        Ok(())
    }
}

// Components
#[derive(Component)]
pub struct OtherPlayer {
    id: u64,
}

// Events
#[derive(Event)]
pub struct OtherPlayerMoved {
    id: u64,
    translation: Vec3,
    animation_frame: usize,
}
#[derive(Event)]
pub struct OtherPlayerDisconnected(u64);

// Systems
/// This system is not responsible for setting MultiplayerState to Online.
/// Whichever system reads the packets should set MultiplayerState::Online when it receives Packet::ClientConnect.
pub(crate) fn setup_client_runtime(
    mut commands: Commands,
    mut next_state: ResMut<NextState<MultiplayerState>>,
) {
    next_state.set(MultiplayerState::Connecting);

    let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
    let (to_client, from_bevy) = mpsc::channel::<Packet>(128);
    let (to_bevy, from_server) = mpsc::channel::<Packet>(128);
    // Connect to server
    let connection_handle = runtime.spawn(async move {
        match connect_to_server(from_bevy, to_bevy).await {
            Ok(output) => Ok(output),
            Err(e) => {
                // Report the error immediately, rather than waiting for the join handle to read it
                error!("Unable to connect to server: {e:#?}");
                Err(e)
            }
        }
    });

    commands.insert_resource(ServerConnection {
        runtime,
        connection_handle,
        to_client,
        from_server,
    });
}

/// This system reads incoming packets, and fires a matching event for each one.
/// This system is responsible for setting MultiplayerState to Online whenever the server says it is connected.
#[tracing::instrument(skip(connection, next_state, player_moved, player_disconnected))]
pub fn read_packets(
    mut connection: ResMut<ServerConnection>,
    mut next_state: ResMut<NextState<MultiplayerState>>,
    mut player_moved: EventWriter<OtherPlayerMoved>,
    mut player_disconnected: EventWriter<OtherPlayerDisconnected>,
) {
    // let time = Instant::now();
    while let Ok(packet) = connection.from_server.try_recv() {
        match packet {
            Packet::ClientConnect => next_state.set(MultiplayerState::Online),
            Packet::ClientDisconnect(id) => match id {
                None => next_state.set(MultiplayerState::Offline),
                Some(id) => {
                    player_disconnected.write(OtherPlayerDisconnected(id));
                }
            },
            Packet::PlayerMovement {
                id,
                x,
                y,
                z,
                animation_frame,
            } => {
                player_moved.write(OtherPlayerMoved {
                    id: id.expect("Server should send id of movement. Please report to dev."),
                    translation: Vec3::new(x, y, z),
                    animation_frame: animation_frame as usize,
                });
            }
        }
    }
    // info!("Took {:?}", time.elapsed());
}

/// This system updates the transforms of other players, and spawns the player if they don't exist yet.
pub fn on_other_player_moved(
    mut commands: Commands,
    assets: Res<OverworldAssetCollection>,
    mut sprite3d_params: Sprite3dParams,
    mut player_moved: EventReader<OtherPlayerMoved>,
    mut query: Query<(&OtherPlayer, &mut Transform, &mut Sprite3d)>,
) {
    for movement in player_moved.read() {
        let mut found_player = false;
        for (other_player, mut transform, mut sprite_3d) in query.iter_mut() {
            if other_player.id == movement.id {
                transform.translation = movement.translation;
                sprite_3d.texture_atlas.as_mut().unwrap().index = movement.animation_frame;
                found_player = true;
            }
        }
        if !found_player {
            commands.spawn((
                StateScoped(MultiplayerState::Online),
                OtherPlayer { id: movement.id },
                Sprite3dBuilder {
                    image: assets.sprites.other_player_image.clone(),
                    pixels_per_metre: SPRITE_PIXELS_PER_METER,
                    double_sided: false,
                    unlit: true,
                    ..default()
                }
                .bundle_with_atlas(
                    &mut sprite3d_params,
                    TextureAtlas {
                        layout: assets.sprites.sprite_layout.clone(),
                        index: movement.animation_frame,
                    },
                ),
                Transform::from_translation(movement.translation),
            ));
        }
    }
}

pub fn on_other_player_disconnected(
    mut commands: Commands,
    mut players_disconnected: EventReader<OtherPlayerDisconnected>,
    query: Query<(&OtherPlayer, Entity)>,
) {
    for player_disconnected in players_disconnected.read() {
        for (other_player, entity) in query.iter() {
            if other_player.id == player_disconnected.0 {
                if let Ok(mut entity) = commands.get_entity(entity) {
                    entity.despawn();
                }
            }
        }
    }
}

pub fn send_current_position(
    connection: Res<ServerConnection>,
    mut next_state: ResMut<NextState<MultiplayerState>>,
    position: Single<(&Velocity, &Transform, &Sprite3d)>,
) {
    let (velocity, transform, sprite_3d) = position.into_inner();
    let velocity = velocity.linvel;

    if velocity.length() != 0.0 {
        let packet = Packet::PlayerMovement {
            id: None,
            x: transform.translation.x,
            y: transform.translation.y,
            z: transform.translation.z,
            animation_frame: u8::try_from(sprite_3d.texture_atlas.as_ref().unwrap().index)
                .expect("Sprite atlas index should fit within 0 and 255"),
        };
        match connection.to_client.try_send(packet) {
            Ok(_) => {}
            Err(TrySendError::Full(_)) => {
                info!("Packet channel is full, packet not sent.");
            }
            Err(TrySendError::Closed(_)) => {
                error!("Packet channel is closed, no longer sending packets.");
                next_state.set(MultiplayerState::Offline);
            }
        }
    }
}

/// A system that tries to disconnect from the server when the window is closed.
/// This should only be called if MultiplayerState is Online.
pub(crate) fn stop_client_runtime_on_window_close(
    mut commands: Commands,
    mut server_connection: ResMut<ServerConnection>,
    mut next_state: ResMut<NextState<MultiplayerState>>,
    mut window_close_requested: EventReader<WindowCloseRequested>,
) {
    for _ in window_close_requested.read() {
        match server_connection.try_disconnect() {
            Ok(()) => {
                info!("Successfully disconnected from server.");
            }
            Err(e) => {
                error!("Unable to disconnect from server: {e:#?}");
            }
        }
        commands.remove_resource::<ServerConnection>();
        next_state.set(MultiplayerState::Offline);
    }
}
