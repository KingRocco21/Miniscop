use crate::states::AppState;
use bevy::prelude::*;
use bevy::text::FontSmoothing;

pub struct MainMenuPlugin;
impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), setup_main_menu);
    }
}

// Constants
const GIFT_ASPECT_RATIO: f32 = 88.0 / 83.0;
const LOGO_COLOR: Color = Color::srgb_u8(255, 105, 255);

// Systems
fn setup_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Music
    // commands.spawn((
    //     StateScoped(AppState::MainMenu),
    //     AudioPlayer::new(asset_server.load("mainmenu/petscop.ogg")),
    // ));
    // Main 2D Camera
    commands.spawn((StateScoped(AppState::MainMenu), Camera2d));
    // Font
    let petscop_font = asset_server.load::<Font>("global/fonts/PetscopWide.ttf");
    // UI
    commands.spawn((
        StateScoped(AppState::MainMenu),
        // Big UI with a width of twice the window width to fit multiple child UIs
        Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Start,
            width: Val::Percent(200.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            position_type: PositionType::Absolute,
            ..default()
        },
        children![
            // Title Node
            (
                Node {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    width: Val::Percent(50.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                children![
                    (
                        ImageNode::new(asset_server.load("mainmenu/gift.png")),
                        Transform::default(),
                        Node {
                            width: Val::Px(GIFT_ASPECT_RATIO * 300.0),
                            height: Val::Px(300.0),
                            position_type: PositionType::Absolute,
                            ..default()
                        }
                    ),
                    (
                        Text::new("Miniscop:"),
                        TextColor::from(LOGO_COLOR),
                        TextFont {
                            font: asset_server.load("global/fonts/PoetsenOne-Regular.ttf"),
                            font_size: 80.0,
                            font_smoothing: FontSmoothing::None,
                            ..default()
                        }
                    ),
                    (
                        Text::new("Investigate Together!"),
                        TextColor::from(LOGO_COLOR),
                        TextFont {
                            font: asset_server.load("global/fonts/PoetsenOne-Regular.ttf"),
                            font_size: 80.0,
                            font_smoothing: FontSmoothing::None,
                            ..default()
                        }
                    ),
                    (
                        Text::new("Press Z to Begin"),
                        TextColor::WHITE,
                        TextFont {
                            font: petscop_font.clone(),
                            font_size: 50.0,
                            font_smoothing: FontSmoothing::None,
                            ..default()
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: Val::Percent(20.0),
                            ..default()
                        }
                    ),
                    (
                        Text::new("© 1997 Garalina"),
                        TextColor::WHITE,
                        TextFont {
                            font: petscop_font,
                            font_size: 60.0,
                            font_smoothing: FontSmoothing::None,
                            ..default()
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: Val::Percent(5.0),
                            ..default()
                        }
                    ),
                ]
            ),
            // Todo: Add rest of UI lol
        ],
    ));
}
