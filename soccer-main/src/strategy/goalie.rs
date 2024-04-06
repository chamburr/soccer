use crate::{
    modules::HEADING_SIGNAL,
    strategy::{
        Data, BALLCAP_DISTANCE, COORDINATE_SIGNAL, FIELD_LENGTH, FIELD_MARGIN, FIELD_MARGIN_Y,
        FIELD_WIDTH,
    },
};
use embassy_time::Instant;
use num_traits::{clamp, Float};

const GOALIE_DISTANCE: f32 = 3.;
const MOVEMENT_THRESHOLD_X: f32 = 7.5;
const MOVEMENT_THRESHOLD_Y: f32 = 7.5;
const CHANGED_THRESHOLD: u64 = 2500;
const MIN_X: f32 = 10.;

pub struct GoalieState {
    pub last_bx: f32,
    pub last_by: f32,
    pub last_changed: Instant,
    pub pushing: bool,
}

impl Default for GoalieState {
    fn default() -> Self {
        Self {
            last_bx: -999.,
            last_by: -999.,
            last_changed: Instant::now(),
            pushing: false,
        }
    }
}

pub async fn run(data: Data, state: &mut GoalieState) {
    let (bx, by, _bok) = data.ball;
    let (x, y, ok) = data.coordinates;

    HEADING_SIGNAL.signal(0.);

    if !ok {
        COORDINATE_SIGNAL.signal((x, y + 5.));
        return;
    }

    if (bx - state.last_bx).abs() > MOVEMENT_THRESHOLD_X
        || (by - state.last_by).abs() > MOVEMENT_THRESHOLD_Y
    {
        state.last_bx = bx;
        state.last_by = by;
        state.last_changed = Instant::now();
    }

    if (x - bx).abs() < BALLCAP_DISTANCE / 2.
        && state.last_changed.elapsed().as_millis() > CHANGED_THRESHOLD
    {
        state.pushing = true;
    }

    let new_x = clamp(bx, FIELD_MARGIN + MIN_X, FIELD_WIDTH - FIELD_MARGIN - MIN_X);
    let new_y = FIELD_LENGTH - FIELD_MARGIN_Y - GOALIE_DISTANCE;

    COORDINATE_SIGNAL.signal((new_x, new_y));
}
