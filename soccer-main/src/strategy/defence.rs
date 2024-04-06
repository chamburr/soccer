use crate::{
    modules::HEADING_SIGNAL,
    strategy::{Data, CLEARANCE_Y, COORDINATE_SIGNAL},
};
use embassy_time::Instant;

const LAST_PUSH_THRESHOLD: u64 = 100;

pub struct DefenceState {
    pub last_push: Instant,
}

impl Default for DefenceState {
    fn default() -> Self {
        Self {
            last_push: Instant::from_millis(0),
        }
    }
}

pub async fn run(data: Data, state: &mut DefenceState) {
    let (bx, by, _) = data.ball;
    let (_, y, _) = data.coordinates;

    HEADING_SIGNAL.signal(0.);

    if y > by {
        state.last_push = Instant::now();
    }

    if state.last_push.elapsed().as_millis() < LAST_PUSH_THRESHOLD {
        COORDINATE_SIGNAL.signal((bx, by + 1.5)); // should +1.5 ??
    } else {
        COORDINATE_SIGNAL.signal((bx, by - CLEARANCE_Y - 2.));
    }
}
