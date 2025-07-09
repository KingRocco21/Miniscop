mod multiplayer;
mod physics;

use crate::AppState;
use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;
use bevy_sprite3d::{Sprite3d, Sprite3dBuilder, Sprite3dParams};
use multiplayer::MultiplayerState;

pub struct OverworldPlugin;
impl Plugin for OverworldPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<OverworldState>()
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
                RunFixedMainLoop,
                physics::handle_input
                    .in_set(RunFixedMainLoopSystem::BeforeFixedMainLoop)
                    .run_if(in_state(OverworldState::InGame)),
            )
            .add_systems(
                FixedUpdate,
                (
                    // These must be run in this order because each one is dependent on the next.
                    multiplayer::read_packets.run_if(
                        in_state(MultiplayerState::Connecting)
                            .or(in_state(MultiplayerState::Online)),
                    ),
                    (
                        multiplayer::on_other_player_moved,
                        multiplayer::on_other_player_disconnected,
                    )
                        .chain()
                        .run_if(in_state(MultiplayerState::Online)),
                    physics::advance_physics,
                    multiplayer::send_current_position.run_if(in_state(MultiplayerState::Online)),
                    animate_sprites,
                )
                    .chain()
                    .run_if(in_state(OverworldState::InGame)),
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
const STARTING_TRANSLATION: Vec3 = Vec3::new(0.0, 64.0 / SPRITE_PIXELS_PER_METER * 0.5, 0.0);
/// Meters per second squared
const ACCELERATION: f32 = 50.0;
const MAX_ACCELERATION_VEC: Vec3 = Vec3::splat(ACCELERATION);
const VELOCITY: f32 = 5.0;
const MAX_VELOCITY_VEC: Vec3 = Vec3::splat(VELOCITY);

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
#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

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
        ));
        // Spawn player
        commands.spawn((
            StateScoped(AppState::Overworld),
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
            Player,
            AnimationTimer(Timer::from_seconds(0.15, TimerMode::Repeating)),
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

// Mod (%) by the column count to find which column the atlas is in.
// Floor divide by the row count to find which row the atlas is in. Multiply by row count to return to that row.
fn animate_sprites(
    mut commands: Commands,
    fixed_time: Res<Time>,
    mut query: Query<(&mut AnimationTimer, &Velocity, &mut Sprite3d)>,
    assets: Res<OverworldAssetCollection>,
) {
    let delta = fixed_time.delta();
    for (mut timer, velocity, mut sprite_3d) in query.iter_mut() {
        let linvel = velocity.linvel;
        let atlas = sprite_3d.texture_atlas.as_mut().unwrap();

        if linvel.xz().length() == 0.0 {
            // Stopped moving, so stop animation in current direction
            timer.pause();
            timer.reset();
            atlas.index = atlas.index % 5;
        } else {
            // Get the current animation frame without direction taken into account.
            // Then update the animation to the current direction.
            // To be faithful to Petscop, left and right overrides forward and backward.
            let current_frame = (atlas.index as f32 / 5.0).floor() as usize * 5;
            if linvel.x < 0.0 {
                // Left
                atlas.index = current_frame + 2;
            } else if linvel.x > 0.0 {
                // Right
                atlas.index = current_frame + 1;
            } else if linvel.z < 0.0 {
                // Forward
                atlas.index = current_frame + 3;
            } else if linvel.z > 0.0 {
                // Backward
                atlas.index = current_frame;
            }

            // If the player just started moving, immediately switch to the first frame, but don't play a sound.
            if timer.paused() {
                timer.unpause();
                // Increment and wrap
                atlas.index += 5;
                if atlas.index > 23 {
                    atlas.index = atlas.index % 5 + 5;
                }
            }

            timer.tick(delta);
            if timer.just_finished() {
                // Increment and wrap
                atlas.index += 5;
                if atlas.index > 23 {
                    atlas.index = atlas.index % 5 + 5;
                }
                // Play walking sound
                let current_frame = (atlas.index as f32 / 5.0).floor() as usize;
                if current_frame == 2 {
                    commands.spawn((
                        StateScoped(AppState::Overworld),
                        AudioPlayer::new(assets.sound_effects.walking_1.clone()),
                        PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            ..default()
                        },
                    ));
                } else if current_frame == 4 {
                    commands.spawn((
                        StateScoped(AppState::Overworld),
                        AudioPlayer::new(assets.sound_effects.walking_2.clone()),
                        PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            ..default()
                        },
                    ));
                }
            }
        }
    }
}
