use crate::{
    modules::{COORDINATE_SIGNAL, HEADING_SIGNAL},
    strategy::Data,
};

const MOVE_DISTANCE: f32 = 10.; // effectively, speed

#[derive(Default)]
pub struct BoundsState {
    pub was_left: bool,
    pub was_right: bool,
    pub was_front: bool,
    pub was_back: bool,
}

pub async fn run(data: Data, state: &mut BoundsState) {
    let (x, y, _) = data.coordinates;
    let (front, left, right, back) = data.lines;

    let new_x = if left || state.was_left {
        state.was_left = true;
        x + MOVE_DISTANCE
    } else if right || state.was_right {
        state.was_right = true;
        x - MOVE_DISTANCE
    } else {
        x
    };
    let new_y = if front || state.was_front {
        state.was_front = true;
        y + MOVE_DISTANCE
    } else if back || state.was_right {
        state.was_back = true;
        y - MOVE_DISTANCE
    } else {
        y
    };

    HEADING_SIGNAL.signal(0.);
    COORDINATE_SIGNAL.signal((new_x, new_y));
}
