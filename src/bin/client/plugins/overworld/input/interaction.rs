use crate::plugins::overworld::animation;
use crate::plugins::overworld::loading::OverworldAssetCollection;
use crate::AppState;
use avian3d::prelude::{OnCollisionEnd, OnCollisionStart};
use bevy::audio::PlaybackMode;
use bevy::prelude::*;

#[derive(Component)]
#[component(immutable)]
pub enum OverworldInteraction {
    Text(String),
}

/// This component is given to the player whenever they can interact with something.
#[derive(Component, Debug)]
#[component(immutable)]
pub struct WithinRangeOfInteractable(pub Entity);

/// This component belongs to interaction sensors, and contains the ID of its interaction prompt.
#[derive(Component, Debug)]
#[component(immutable)]
pub struct InteractableWithPrompt(pub Entity);

pub fn when_approaching_interactable(
    trigger: Trigger<OnCollisionStart>,
    mut commands: Commands,
    assets: Res<OverworldAssetCollection>,
    prompt_query: Query<&InteractableWithPrompt>,
    mut prompt_state_query: Query<&mut animation::AnimatedInteractionPromptState>,
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

    // Get the prompt entity from the trigger target
    let prompt_entity = prompt_query
        .get(trigger.target())
        .expect("The sensor should have an InteractableWithPrompt component")
        .0;
    // Change the AnimatedInteractionPromptState of the prompt entity
    let mut prompt_state = prompt_state_query
        .get_mut(prompt_entity)
        .expect("The interaction prompt should have an AnimatedInteractionPromptState component");
    *prompt_state = animation::AnimatedInteractionPromptState::Growing;

    // Add the "WithinRangeOfInteractable" component to the player
    commands
        .entity(trigger.collider)
        .insert(WithinRangeOfInteractable(trigger.target()));
}

pub fn when_leaving_interactable(
    trigger: Trigger<OnCollisionEnd>,
    mut commands: Commands,
    prompt_query: Query<&InteractableWithPrompt>,
    mut prompt_state_query: Query<&mut animation::AnimatedInteractionPromptState>,
) {
    // Get the prompt entity from the trigger target
    let prompt_entity = prompt_query
        .get(trigger.target())
        .expect("The sensor should have an InteractableWithPrompt component")
        .0;
    // Change the "AnimatedInteractionPromptState" of the prompt entity
    let mut prompt_state = prompt_state_query
        .get_mut(prompt_entity)
        .expect("The interaction prompt should have an AnimatedInteractionPromptState component");
    *prompt_state = animation::AnimatedInteractionPromptState::Shrinking;

    // Remove the "WithinRangeOfInteractable" component from the player
    commands
        .entity(trigger.collider)
        .remove::<WithinRangeOfInteractable>();
}
