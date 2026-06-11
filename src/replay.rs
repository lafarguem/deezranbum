use crate::{QueueBehaviours, album::handle_queue, storage::load_state};
use std::cmp;

pub fn replay(from: Option<usize>, to: Option<usize>) {
    let state = load_state();

    let from = cmp::min(from.unwrap_or(0usize), state.album_order.len());

    let to = cmp::min(
        to.map(|t| t + 1).unwrap_or(state.album_order.len()),
        state.album_order.len(),
    );

    for index in from..to {
        let album_id = state.album_order.get(index);
        if let Some(id) = album_id {
            handle_queue(id, QueueBehaviours::True, false)
        }
    }
}
