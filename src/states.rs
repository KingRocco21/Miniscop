use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
#[states(scoped_entities)]
pub enum AppState {
    #[default]
    Garalina,
    MainMenu,
    LoadingScreen,
}
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(AppState = AppState::MainMenu)]
pub enum MainMenuState {
    #[default]
    Title,
    FileSelect,
    FileNaming,
}
