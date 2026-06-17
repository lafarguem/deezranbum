use crate::{
    error::AppResult,
    storage::{load_state, save_state},
};

pub fn set(user_id: String) -> AppResult<()> {
    let mut state = load_state()?;
    state.user_id = user_id;
    save_state(&state)
}
