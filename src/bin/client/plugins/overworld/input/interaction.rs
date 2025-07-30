use crate::plugins::overworld::animation;
use crate::plugins::overworld::loading::OverworldAssetCollection;
use crate::{AppState, PetscopFont};
use avian3d::prelude::{OnCollisionEnd, OnCollisionStart};
use bevy::audio::PlaybackMode;
use bevy::prelude::*;
use std::collections::VecDeque;
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;

/// This component is given to interactable objects created in blender.
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

enum CustomGrapheme {
    Grapheme {
        grapheme: String,
        color: Color,
        /// Whether to pause for a brief period after displaying this grapheme.
        precedes_pause: bool,
    },
    NewLine,
    EndOfParagraph,
}

/// This component is given to UI text boxes, and contains a copy of the text from whichever text interaction the player starts.
/// This text is used to spawn TextSpans that display the text character by character.
/// This is also known as the "typewriter effect."
#[derive(Component)]
pub struct CompleteText {
    graphemes: VecDeque<CustomGrapheme>,
    typewriter_timer: Timer,
    punctuation_pause_timer: Timer,
}

impl CompleteText {
    pub fn new(text: &str) -> Self {
        let mut char_color = Color::BLACK;
        let graphemes = text
            .graphemes(true)
            .map(|g| {
                // https://codepoints.net/U+2028
                if g == "\u{2028}" {
                    CustomGrapheme::NewLine
                }
                // https://codepoints.net/U+2029
                else if g == "\u{2029}" {
                    CustomGrapheme::EndOfParagraph
                } else {
                    let grapheme = String::from(g);
                    let color = char_color;
                    let precedes_pause = g == "," || g == "?" || g == "!";
                    CustomGrapheme::Grapheme {
                        grapheme,
                        color,
                        precedes_pause,
                    }
                }
            })
            .collect::<VecDeque<CustomGrapheme>>();
        Self {
            graphemes,
            typewriter_timer: Timer::new(Duration::from_millis(50), TimerMode::Repeating),
            punctuation_pause_timer: Timer::new(Duration::from_millis(250), TimerMode::Once),
        }
    }
}

pub fn typewrite_text(
    mut commands: Commands,
    text_node: Single<(Entity, &mut CompleteText)>,
    time: Res<Time>,
    petscop_font: Res<PetscopFont>,
    text_box_node: Single<Entity, With<TextBoxNode>>,
    assets: Res<OverworldAssetCollection>,
) {
    let (text_node, mut text) = text_node.into_inner();
    let delta = time.delta();
    if text.typewriter_timer.paused() {
        text.punctuation_pause_timer.tick(delta);
        if text.punctuation_pause_timer.just_finished() {
            text.punctuation_pause_timer.reset();
            text.typewriter_timer.unpause();
        }
    }
    text.typewriter_timer.tick(delta);
    if text.typewriter_timer.just_finished() {
        match text.graphemes.pop_front() {
            None => {
                // There are no more characters to typewrite, so the component can be removed.
                commands.entity(text_node).remove::<CompleteText>();
            }
            Some(grapheme) => match grapheme {
                CustomGrapheme::Grapheme {
                    grapheme,
                    color,
                    precedes_pause,
                } => {
                    let next_text_span = commands
                        .spawn((
                            TextSpan(grapheme),
                            petscop_font.clone().with_font_size(48.0),
                            TextColor(color),
                        ))
                        .id();
                    commands.entity(text_node).add_child(next_text_span);
                    if precedes_pause {
                        text.typewriter_timer.pause();
                    }
                }
                CustomGrapheme::NewLine => {
                    let next_text_span = commands.spawn(TextSpan::from("\n")).id();
                    commands.entity(text_node).add_child(next_text_span);
                }
                CustomGrapheme::EndOfParagraph => {
                    let arrow_node = commands
                        .spawn((
                            ImageNode::new(assets.sprites.text_box_arrow_image.clone()),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(0.0),
                                bottom: Val::Percent(0.0),
                                ..default()
                            },
                        ))
                        .id();
                    commands.entity(*text_box_node).add_child(arrow_node);
                    text.typewriter_timer.pause();
                    text.punctuation_pause_timer.pause();
                }
            },
        }
    }
}

#[derive(Component)]
pub struct TextBoxNode;
