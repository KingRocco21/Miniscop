use crate::plugins::overworld::OverworldAssetCollection;
use crate::AppState;
use bevy::audio::{AudioPlayer, PlaybackMode, PlaybackSettings};
use bevy::math::{Vec3, Vec3Swizzles};
use bevy::prelude::{Commands, Component, Deref, DerefMut, Query, Res, StateScoped};
use bevy::time::{Time, Timer};
use bevy::utils::default;
use bevy_sprite3d::Sprite3d;

// Components
#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);
#[derive(Component, Deref, DerefMut)]
pub struct AnimationDirection(pub Vec3);

// Systems
// Mod (%) by the column count to find which column the atlas is in.
// Floor divide by the row count to find which row the atlas is in. Multiply by row count to return to that row.
pub fn animate_sprites(
    mut commands: Commands,
    fixed_time: Res<Time>,
    mut query: Query<(&mut AnimationTimer, &AnimationDirection, &mut Sprite3d)>,
    assets: Res<OverworldAssetCollection>,
) {
    let delta = fixed_time.delta();
    for (mut timer, direction, mut sprite_3d) in query.iter_mut() {
        let direction = direction.0;

        let atlas = sprite_3d.texture_atlas.as_mut().unwrap();

        if direction.xz().length() < 0.001 {
            // Stopped moving, so stop animation in current direction
            timer.pause();
            timer.reset();
            atlas.index = atlas.index % 5;
        } else {
            // Get the current animation frame without direction taken into account.
            // Then update the animation to the current direction.
            // To be faithful to Petscop, left and right overrides forward and backward.
            let current_frame = (atlas.index as f32 / 5.0).floor() as usize * 5;
            if direction.x < -0.001 {
                // Left
                atlas.index = current_frame + 2;
            } else if direction.x > 0.001 {
                // Right
                atlas.index = current_frame + 1;
            } else if direction.z < -0.001 {
                // Forward
                atlas.index = current_frame + 3;
            } else if direction.z > 0.001 {
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
