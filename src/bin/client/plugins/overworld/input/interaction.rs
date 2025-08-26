use crate::plugins::overworld::input::{PlayerAction, TextAction};
use crate::plugins::overworld::loading::OverworldAssetCollection;
use crate::plugins::overworld::{animation, Player};
use crate::{AppState, PetscopFont};
use avian3d::prelude::{OnCollisionEnd, OnCollisionStart};
use bevy::audio::PlaybackMode;
use bevy::prelude::*;
use leafwing_input_manager::action_state::ActionState;
use std::collections::VecDeque;
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;

/// This component is given to interactable objects created in blender.
#[derive(Component, Clone)]
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

#[derive(Component)]
pub struct ScreenContainerNode;
#[derive(Component)]
pub struct TextBoxNode;
#[derive(Component)]
pub struct TextBoxArrowNode;

/// Due to the way Bevy works, this system will only run when the player has the "WithinRangeOfInteractable" component.
pub fn interact(
    within_range_of: Single<&WithinRangeOfInteractable, With<Player>>,
    mut player_action: ResMut<ActionState<PlayerAction>>,
    interactions: Query<&OverworldInteraction>,
    mut commands: Commands,
    assets: Res<OverworldAssetCollection>,
) {
    let within_range_of = within_range_of.into_inner();

    if player_action.just_pressed(&PlayerAction::Interact) {
        let interaction = interactions
            .get(within_range_of.0)
            .expect("Interactable entities should always have interaction data");

        player_action.disable();

        match interaction {
            OverworldInteraction::Text(text) => {
                // Entire screen
                let container_node = (
                    ScreenContainerNode,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                );

                let text_box_node = (
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(9.375),
                        bottom: Val::Percent(12.5),
                        width: Val::Vw(81.25),
                        height: Val::Vh(28.75),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ImageNode {
                        image: assets.sprites.text_box_image.clone(),
                        image_mode: NodeImageMode::Sliced(assets.sprites.text_box_slicer.clone()),
                        ..default()
                    },
                    TextBoxNode,
                );

                let text_node = (
                    Node {
                        width: Val::Vw(75.0),
                        height: Val::Vh(20.0),
                        ..default()
                    },
                    Text::default(),
                    CompleteText::new(text),
                );

                let mut arrow_animation_timer =
                    Timer::new(Duration::from_millis(500), TimerMode::Repeating);
                // Arrow starts off without animation
                arrow_animation_timer.pause();

                let arrow_node = (
                    TextBoxArrowNode,
                    ImageNode::new(assets.sprites.text_box_arrow_image.clone()),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Vw(75.0),
                        bottom: Val::Vh(5.0),
                        ..default()
                    },
                    Transform::from_scale(Vec3::splat(3.0)),
                    animation::AnimationTimer(arrow_animation_timer),
                    Visibility::Hidden,
                );

                commands.spawn((
                    StateScoped(AppState::Overworld),
                    container_node,
                    children![(text_box_node, children![text_node, arrow_node])],
                ));
            }
        }
    }
}

#[derive(Debug)]
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

/// Pops a grapheme.
/// Returns whether any further graphemes should be popped.
/// If false, you should make the arrow visible.
fn pop_grapheme(
    text: &mut CompleteText,
    commands: &mut Commands,
    text_node: &Entity,
    petscop_font: &PetscopFont,
) -> bool {
    match text.graphemes.pop_front() {
        Some(grapheme) => match grapheme {
            CustomGrapheme::Grapheme {
                grapheme,
                color,
                precedes_pause,
            } => {
                let next_text_span = commands
                    .spawn((
                        TextSpan(grapheme),
                        (*petscop_font).clone().with_font_size(48.0),
                        TextColor(color),
                    ))
                    .id();
                commands.entity(*text_node).add_child(next_text_span);
                if precedes_pause {
                    text.typewriter_timer.pause();
                }
                true
            }
            CustomGrapheme::NewLine => {
                let next_text_span = commands.spawn(TextSpan::from("\n")).id();
                commands.entity(*text_node).add_child(next_text_span);
                true
            }
            CustomGrapheme::EndOfParagraph => {
                text.typewriter_timer.pause();
                text.punctuation_pause_timer.pause();
                false
            }
        },
        None => {
            // info!("No more graphemes.");
            // No more graphemes, so the text box will close the next time the player interacts.
            false
        }
    }
}
pub fn proceed_text(
    mut commands: Commands,
    mut player_action: ResMut<ActionState<PlayerAction>>,
    mut text_action: ResMut<ActionState<TextAction>>,
    screen_container_node: Single<Entity, With<ScreenContainerNode>>,
    text_node: Single<(Entity, &mut CompleteText)>,
    arrow_node: Single<(&mut Visibility, &mut animation::AnimationTimer), With<TextBoxArrowNode>>,
    petscop_font: Res<PetscopFont>,
) {
    // This prevents the player's interaction from opening the text box AND skipping to the end of the paragraph on the same frame.
    if text_action.disabled() {
        text_action.enable();
        return;
    }
    if text_action.just_pressed(&TextAction::Proceed) {
        let screen_container_node = screen_container_node.into_inner();
        let (text_node, mut text) = text_node.into_inner();
        // There are three possibilities:
        // 1. There is no more text left to display
        // 2. The end of a paragraph was previously reached, and the text and arrow needs to be cleared.
        // 3. The text is currently typewriting, and needs to skip to the end of the paragraph.
        if text.graphemes.is_empty() {
            // info!("No more graphemes, despawning");
            commands.entity(screen_container_node).despawn();
            text_action.disable();
            player_action.enable();
        } else if text.typewriter_timer.paused() && text.punctuation_pause_timer.paused() {
            // info!("Starting new paragraph");
            commands.entity(text_node).despawn_related::<Children>();
            // Make arrow invisible and prevent it from flickering
            let (mut arrow_visibility, mut arrow_timer) = arrow_node.into_inner();
            *arrow_visibility = Visibility::Hidden;
            arrow_timer.pause();

            text.typewriter_timer.unpause();
            text.punctuation_pause_timer.unpause();
        } else {
            // info!("Popping next grapheme: {:?}", text.graphemes.get(0));
            while pop_grapheme(&mut *text, &mut commands, &text_node, &petscop_font) {}
            // Make arrow visible and allow it to start flickering
            let (mut arrow_visibility, mut arrow_timer) = arrow_node.into_inner();
            *arrow_visibility = Visibility::Visible;
            arrow_timer.reset();
            arrow_timer.unpause();
        }
    }
}

pub fn typewrite_text(
    mut commands: Commands,
    text_node: Single<(Entity, &mut CompleteText)>,
    arrow_node: Single<(&mut Visibility, &mut animation::AnimationTimer), With<TextBoxArrowNode>>,
    time: Res<Time>,
    petscop_font: Res<PetscopFont>,
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
        if !pop_grapheme(&mut *text, &mut commands, &text_node, &petscop_font) {
            // Make arrow visible and allow it to start flickering
            let (mut arrow_visibility, mut arrow_timer) = arrow_node.into_inner();
            *arrow_visibility = Visibility::Visible;
            arrow_timer.reset();
            arrow_timer.unpause();
        }
    }
}
