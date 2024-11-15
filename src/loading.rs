use bevy::prelude::*;

use crate::state::AppState;

pub fn loading_complete(mut next_state: ResMut<NextState<AppState>>) {
    next_state.set(AppState::InGame);
}
