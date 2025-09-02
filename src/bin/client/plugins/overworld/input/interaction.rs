use crate::plugins::overworld::input::{PlayerAction, TextAction};
use crate::plugins::overworld::{animation, loading, OverworldState, Player};
use crate::{AppState, PetscopFont};
use avian3d::prelude::{OnCollisionEnd, OnCollisionStart};
use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;
use bevy_asset_loader::dynamic_asset::DynamicAssets;
use bevy_asset_loader::prelude::StandardDynamicAsset;
use leafwing_input_manager::action_state::ActionState;
use std::collections::VecDeque;
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;

/// This component is given to interactable objects created in blender.
#[derive(Component, Reflect, Clone)]
#[component(immutable)]
#[reflect(Component)]
pub enum OverworldInteraction {
    /// https://codepoints.net/U+2028 == New line
    ///
    /// https://codepoints.net/U+2029 == End of paragraph (not needed at the very end of the text)
    Text(String),
    /// The string is case-sensitive and must match the file names of the .gltf and .ogg exactly.
    LevelTransition { next_level: String },
}
/// This component is given to the player whenever they can interact with something.
#[derive(Component, Debug)]
#[component(immutable)]
pub struct WithinRangeOfInteractable(pub Entity);

pub fn when_approaching_interactable(
    trigger: Trigger<OnCollisionStart>,
    interaction: Query<&OverworldInteraction>,
    mut commands: Commands,
    sounds: Res<loading::OverworldAssets>,
    children_query: Query<&Children>,
    mut prompt_state_query: Query<&mut animation::AnimatedInteractionPromptState>,
    mut dynamic_assets: ResMut<DynamicAssets>,
) {
    if let Ok(interaction) = interaction.get(trigger.target()) {
        match interaction {
            OverworldInteraction::Text(_) => {
                // Play sound
                commands.spawn((
                    StateScoped(OverworldState::InScene),
                    AudioPlayer::new(sounds.approaching_interactable.clone()),
                    PlaybackSettings {
                        mode: PlaybackMode::Despawn,
                        ..default()
                    },
                ));

                // Add "WithinRangeOfInteractable" to player
                commands
                    .entity(trigger.collider)
                    .insert(WithinRangeOfInteractable(trigger.target()));

                // Change the AnimatedInteractionPromptState of the prompt entity
                for child in children_query.iter_descendants(trigger.target()) {
                    if let Ok(mut interaction) = prompt_state_query.get_mut(child) {
                        *interaction = animation::AnimatedInteractionPromptState::Growing;
                    }
                }
            }
            OverworldInteraction::LevelTransition { next_level } => {
                let glb_path = format!("overworld/3d/{next_level}.glb");
                let ogg_path = format!("overworld/music/{next_level}.ogg");
                info!("Attempting to load path {} and {}", glb_path, ogg_path);

                dynamic_assets.register_asset(
                    "gltf",
                    Box::new(StandardDynamicAsset::File { path: glb_path }),
                );
                dynamic_assets.register_asset(
                    "music",
                    Box::new(StandardDynamicAsset::File { path: ogg_path }),
                );

                commands.spawn((
                    StateScoped(OverworldState::InScene),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                    GlobalZIndex(1),
                    animation::FadeOut,
                ));
            }
        }
    }
}

pub fn when_leaving_interactable(
    trigger: Trigger<OnCollisionEnd>,
    mut commands: Commands,
    children_query: Query<&Children>,
    mut prompt_state_query: Query<&mut animation::AnimatedInteractionPromptState>,
) {
    // Remove "WithinRangeOfInteractable" from player
    commands
        .entity(trigger.collider)
        .remove::<WithinRangeOfInteractable>();

    // Change the AnimatedInteractionPromptState of the prompt entity
    for child in children_query.iter_descendants(trigger.target()) {
        if let Ok(mut interaction) = prompt_state_query.get_mut(child) {
            *interaction = animation::AnimatedInteractionPromptState::Shrinking;
        }
    }
}

#[derive(Component)]
#[component(immutable)]
pub struct TextBoxNode;
#[derive(Component)]
#[component(immutable)]
pub struct TextBoxArrowNode;
#[derive(Component)]
#[component(immutable)]
pub struct DialogueSfx;

/// Due to the way Bevy works, this system will only run when the player has the "WithinRangeOfInteractable" component.
pub fn interact(
    within_range_of: Single<&WithinRangeOfInteractable, With<Player>>,
    mut player_action: ResMut<ActionState<PlayerAction>>,
    interactions: Query<&OverworldInteraction>,
    mut commands: Commands,
    assets: Res<loading::OverworldAssets>,
) {
    if player_action.just_pressed(&PlayerAction::Interact) {
        let within_range_of = within_range_of.into_inner();

        let interaction = interactions
            .get(within_range_of.0)
            .expect("Interactable entities should always have OverworldInteraction");

        match interaction {
            OverworldInteraction::Text(text) => {
                player_action.disable();
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
                        image: assets.text_box_image.clone(),
                        image_mode: NodeImageMode::Sliced(TextureSlicer {
                            border: BorderRect {
                                left: 8.0,
                                bottom: 7.0,
                                right: 8.0,
                                top: 7.0,
                            },
                            max_corner_scale: 3.0,
                            ..default()
                        }),
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
                    ImageNode::new(assets.text_box_arrow_image.clone()),
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

                // Play dialogue sound as a child of the interaction
                // (It needs to be despawned when the interaction ends)
                let dialogue_sfx = (
                    DialogueSfx,
                    AudioPlayer::new(assets.dialogue.clone()),
                    PlaybackSettings {
                        mode: PlaybackMode::Loop,
                        volume: Volume::Linear(1.0),
                        ..default()
                    },
                );

                commands.spawn((
                    StateScoped(AppState::Overworld),
                    text_box_node,
                    children![text_node, arrow_node, dialogue_sfx],
                ));

                // Play dialogue start sound
                commands.spawn((
                    StateScoped(AppState::Overworld),
                    AudioPlayer::new(assets.dialogue_start.clone()),
                    PlaybackSettings {
                        mode: PlaybackMode::Despawn,
                        volume: Volume::Linear(1.0),
                        ..default()
                    },
                ));
            }
            OverworldInteraction::LevelTransition { .. } => return, // No action on interact
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
///
/// This text is used to spawn TextSpans that display the text character by character.
/// This is also known as the "typewriter effect."
#[derive(Component)]
pub struct CompleteText {
    /// A queue of all graphemes to be displayed from the text interaction.
    graphemes: VecDeque<CustomGrapheme>,
    /// The duration to wait before showing the next grapheme.
    typewriter_timer: Timer,
    /// The duration to wait after punctuation.
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
                    CustomGrapheme::Grapheme {
                        grapheme: String::from(g),
                        color: char_color,
                        precedes_pause: g == "," || g == "?" || g == "!",
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

pub fn proceed_text(
    mut commands: Commands,
    mut player_action: ResMut<ActionState<PlayerAction>>,
    mut text_action: ResMut<ActionState<TextAction>>,
    text_box_node: Single<Entity, With<TextBoxNode>>,
    text_node: Single<(Entity, &mut CompleteText)>,
    dialogue_sfx: Single<&AudioSink, With<DialogueSfx>>,
    arrow_node: Single<(&mut Visibility, &mut animation::AnimationTimer), With<TextBoxArrowNode>>,
    petscop_font: Res<PetscopFont>,
    assets: Res<loading::OverworldAssets>,
) {
    // This prevents the player's interaction from opening the text box AND skipping to the end of the paragraph on the same frame.
    if text_action.disabled() {
        text_action.enable();
        return;
    }
    if text_action.just_pressed(&TextAction::Proceed) {
        let text_box_node = text_box_node.into_inner();
        let (text_node, mut text) = text_node.into_inner();
        // There are three possibilities:
        // 1. There is no more text left to display.
        //      The dialogue end sfx needs to play,
        //      and the text interaction needs to end.
        // 2. The end of a paragraph was previously reached.
        //      The dialogue start sfx needs to play,
        //      the dialogue sfx needs to be unpaused,
        //      and the text and arrow need to be cleared so new graphemes can be displayed.
        // 3. The text is currently typewriting, and needs to skip to the end of the paragraph.
        if text.graphemes.is_empty() {
            // info!("No more graphemes, despawning");

            // Play dialogue end sound
            commands.spawn((
                StateScoped(AppState::Overworld),
                AudioPlayer::new(assets.dialogue_end.clone()),
                PlaybackSettings {
                    mode: PlaybackMode::Despawn,
                    volume: Volume::Linear(1.0),
                    ..default()
                },
            ));

            commands.entity(text_box_node).despawn();
            text_action.disable();
            player_action.enable();
        } else if text.typewriter_timer.paused() && text.punctuation_pause_timer.paused() {
            // info!("Starting new paragraph");
            commands.entity(text_node).despawn_related::<Children>();

            // Play dialogue start sound
            commands.spawn((
                StateScoped(AppState::Overworld),
                AudioPlayer::new(assets.dialogue_start.clone()),
                PlaybackSettings {
                    mode: PlaybackMode::Despawn,
                    volume: Volume::Linear(1.0),
                    ..default()
                },
            ));

            // Play dialogue sfx
            dialogue_sfx.play();

            // Make arrow invisible and prevent it from flickering
            let (mut arrow_visibility, mut arrow_timer) = arrow_node.into_inner();
            *arrow_visibility = Visibility::Hidden;
            arrow_timer.pause();

            text.typewriter_timer.unpause();
            text.punctuation_pause_timer.unpause();
        } else {
            // info!("Popping next grapheme: {:?}", text.graphemes.get(0));
            let (mut arrow_visibility, mut arrow_timer) = arrow_node.into_inner();
            while pop_grapheme(
                &mut *text,
                &mut commands,
                &text_node,
                &*dialogue_sfx,
                &mut *arrow_visibility,
                &mut *arrow_timer,
                &petscop_font,
            ) {}
        }
    }
}
pub fn typewrite_text(
    mut commands: Commands,
    text_node: Single<(Entity, &mut CompleteText)>,
    dialogue_sfx: Single<&AudioSink, With<DialogueSfx>>,
    arrow_node: Single<(&mut Visibility, &mut animation::AnimationTimer), With<TextBoxArrowNode>>,
    time: Res<Time>,
    petscop_font: Res<PetscopFont>,
) {
    let delta = time.delta();

    let (text_node, mut text) = text_node.into_inner();
    if text.typewriter_timer.paused() {
        text.punctuation_pause_timer.tick(delta);
        if text.punctuation_pause_timer.just_finished() {
            text.punctuation_pause_timer.reset();
            text.typewriter_timer.unpause();

            dialogue_sfx.play();
        }
    }

    text.typewriter_timer.tick(delta);
    if text.typewriter_timer.just_finished() {
        let (mut arrow_visibility, mut arrow_timer) = arrow_node.into_inner();
        pop_grapheme(
            &mut *text,
            &mut commands,
            &text_node,
            &*dialogue_sfx,
            &mut *arrow_visibility,
            &mut *arrow_timer,
            &petscop_font,
        );
    }
}

/// Pops the next grapheme from the front of the queue.
///
/// If the grapheme is a pause or a paragraph end, or if the queue is empty,
/// the dialogue sfx will pause.
///
/// If the grapheme is a paragraph end or the queue is empty,
/// the arrow will be made visible so it can begin flickering.
///
/// Returns whether any further graphemes should be popped.
fn pop_grapheme(
    text: &mut CompleteText,
    commands: &mut Commands,
    text_node: &Entity,
    dialogue_sfx: &AudioSink,
    arrow_visibility: &mut Visibility,
    arrow_timer: &mut animation::AnimationTimer,
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
                    dialogue_sfx.pause();
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

                dialogue_sfx.pause();

                *arrow_visibility = Visibility::Visible;
                arrow_timer.reset();
                arrow_timer.unpause();
                false
            }
        },
        None => {
            // info!("No more graphemes.");
            // No more graphemes, so the text box will close the next time the player interacts.
            dialogue_sfx.pause();

            *arrow_visibility = Visibility::Visible;
            arrow_timer.reset();
            arrow_timer.unpause();
            false
        }
    }
}
