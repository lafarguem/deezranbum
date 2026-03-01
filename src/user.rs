use crate::storage::{save_state, load_state};

pub fn set(user_id: String) -> std::io::Result<()> {
    let mut state = load_state();
    state.user_id = user_id;
    save_state(&state)
}
