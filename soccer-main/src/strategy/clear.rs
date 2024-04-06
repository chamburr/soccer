use crate::{
    constants::BALLCAP_DISTANCE,
    modules::{COORDINATE_SIGNAL, HEADING_SIGNAL},
    strategy::Data,
};
use embassy_time::Instant;
use num_traits::Float;

const PUSH_DISTANCE: f32 = 20.;
const PUSH_DURATION: u64 = 250;
const WAIT_DISTANCE: f32 = 20.;
const WAIT_DURATION: u64 = 2000;

pub struct ClearState {
    pushed: bool,
    pushed_time: Instant,
    waiting: bool,
    waiting_time: Instant,
    moving_x: bool,
}

impl Default for ClearState {
    fn default() -> Self {
        Self {
            pushed: false,
            pushed_time: Instant::now(),
            waiting: false,
            waiting_time: Instant::now(),
            moving_x: false,
        }
    }
}

pub async fn run(data: Data, state: &mut ClearState) {
    let (x, y, _) = data.coordinates;
    let (bx, by, _) = data.ball;

    HEADING_SIGNAL.signal(0.);

    if state.pushed && state.pushed_time.elapsed().as_millis() > PUSH_DURATION {
        if !state.waiting {
            state.waiting = true;
            state.waiting_time = Instant::now();
        }

        if state.waiting && state.waiting_time.elapsed().as_millis() < WAIT_DURATION {
            COORDINATE_SIGNAL.signal((bx, by + WAIT_DISTANCE));
            return;
        } else {
            state.pushed = false;
            state.waiting = false;
        }
    }

    let new_x = bx;
    let new_y = if (!state.pushed && y - by < PUSH_DISTANCE - 2.)
        || (!state.moving_x && (x - bx).abs() > BALLCAP_DISTANCE)
        || (state.moving_x && (x - bx).abs() > BALLCAP_DISTANCE / 2.)
    {
        state.moving_x = true;
        by - PUSH_DISTANCE - BALLCAP_DISTANCE
    } else {
        state.moving_x = false;
        by - BALLCAP_DISTANCE - 3.
    };

    if !state.moving_x && !state.pushed {
        state.pushed = true;
        state.pushed_time = Instant::now();
    }

    COORDINATE_SIGNAL.signal((new_x, new_y));
}
