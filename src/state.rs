use bevy::prelude::States;

#[derive(States, Debug, Default, Hash, Clone, PartialEq, Eq)]
pub enum AppState {
    #[default]
    PrepareAssets,
    Loading,
    MainMenu,
    InGame,
}
