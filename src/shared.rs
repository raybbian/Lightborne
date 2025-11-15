use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    InGame,
    Ui,
    Loading,
}

#[derive(SubStates, Default, Debug, Clone, PartialEq, Eq, Hash)]
#[source(GameState = GameState::InGame)]
pub enum PlayState {
    #[default]
    Playing,
    Paused,
    Animating,
}

#[derive(SubStates, Default, Debug, Clone, PartialEq, Eq, Hash)]
#[source(PlayState = PlayState::Animating)]
pub enum AnimationState {
    #[default]
    Frozen,
    InputLocked,
}

#[derive(SubStates, Default, Debug, Clone, PartialEq, Eq, Hash)]
#[source(GameState = GameState::Ui)]
pub enum UiState {
    #[default]
    LevelSelect,
    Leaderboard,
    Settings,
    StartMenu,
}

#[derive(Event)]
pub struct ResetLevels;
