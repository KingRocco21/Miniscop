use crate::plugins::overworld::input::PlayerAction;
use crate::plugins::overworld::OverworldAssetCollection;
use crate::AppState;
use avian3d::math::PI;
use bevy::audio::{AudioPlayer, PlaybackMode, PlaybackSettings};
use bevy::prelude::ops::sin;
use bevy::prelude::*;
use bevy::time::{Time, Timer};
use bevy::utils::default;
use bevy_sprite3d::Sprite3d;
use leafwing_input_manager::prelude::ActionState;

// Components
#[derive(Component, Deref, DerefMut)]
pub struct InitialTransform(pub Transform);
#[derive(Component, Eq, PartialEq, Copy, Clone)]
pub enum AnimatedInteractionPromptState {
    Hidden,
    Growing,
    Revealed,
    Shrinking,
}

#[derive(Component, Deref, DerefMut)]
pub struct WalkCycleTimer(pub Timer);

// Systems
// Mod (%) by the column count to find which column the atlas is in.
// Floor divide by the row count to find which row the atlas is in. Multiply by row count to return to that row.
pub fn animate_walk_cycles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(
        &mut WalkCycleTimer,
        &ActionState<PlayerAction>,
        &mut Sprite3d,
    )>,
    assets: Res<OverworldAssetCollection>,
) {
    let delta = time.delta();
    for (mut timer, action_state, mut sprite_3d) in query.iter_mut() {
        let direction = action_state.axis_pair(&PlayerAction::Walk);

        let atlas = sprite_3d.texture_atlas.as_mut().unwrap();

        if direction.length() == 0.0 {
            // Stopped moving, so stop animation in current direction
            timer.pause();
            timer.reset();
            atlas.index = atlas.index % 5;
        } else {
            // Get the current animation frame without direction taken into account.
            // Then update the animation to the current direction.
            // To be faithful to Petscop, left and right overrides forward and backward.
            let current_frame = (atlas.index as f32 / 5.0).floor() as usize * 5;
            if direction.x < 0.0 {
                // Left
                atlas.index = current_frame + 2;
            } else if direction.x > 0.0 {
                // Right
                atlas.index = current_frame + 1;
            } else if direction.y < 0.0 {
                // Forward
                atlas.index = current_frame + 3;
            } else if direction.y > 0.0 {
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

pub fn animate_interaction_prompts(
    time: Res<Time>,
    mut query: Query<(
        &mut AnimatedInteractionPromptState,
        &mut Transform,
        &InitialTransform,
    )>,
) {
    for (mut prompt_state, mut transform, initial_transform) in query.iter_mut() {
        // Grow/shrink prompts
        let delta_seconds = time.delta_secs();
        match *prompt_state {
            AnimatedInteractionPromptState::Growing => {
                // It takes 0.25 seconds to grow to full size
                transform.scale =
                    (transform.scale + Vec3::splat(delta_seconds * 4.0)).min(Vec3::ONE);
                if transform.scale == Vec3::ONE {
                    *prompt_state = AnimatedInteractionPromptState::Revealed;
                }
            }
            AnimatedInteractionPromptState::Shrinking => {
                // It takes 0.25 seconds to shrink to zero
                transform.scale =
                    (transform.scale - Vec3::splat(delta_seconds * 4.0)).max(Vec3::ZERO);
                if transform.scale == Vec3::ZERO {
                    *prompt_state = AnimatedInteractionPromptState::Hidden;
                }
            }
            _ => {}
        }
        // Make visible prompts bob up and down
        if *prompt_state != AnimatedInteractionPromptState::Hidden {
            let seconds = time.elapsed_secs();

            let y_offset = sin(5.0 * seconds) / 4.0;
            transform.translation.y = initial_transform.translation.y + y_offset;

            // 10 degrees max in each direction
            let theta_y = sin(PI * seconds) * PI / 18.0;
            transform.rotation = initial_transform.rotation * Quat::from_rotation_y(theta_y);
        }
    }
}
