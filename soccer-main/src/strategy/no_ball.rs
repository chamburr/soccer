use crate::{
    modules::{HEADING_MUTEX, HEADING_SIGNAL},
    strategy::{Data, COORDINATE_SIGNAL, FIELD_LENGTH, FIELD_MARGIN_Y, FIELD_WIDTH},
    utils::{construct_vector, read_mutex},
};

const NO_BALL_DISTANCE: f32 = 35.;
const GOALIE_NO_BALL_DISTANCE: f32 = 5.;
const MAX_HEADING: f32 = 30.;
const DISTANCE_THRESHOLD: f32 = 15.;

#[derive(Default)]
pub struct NoBallState {
    pub clockwise: bool,
}

pub async fn run(data: Data, state: &mut NoBallState) {
    let (x, y, ok) = data.coordinates;
    let goalie = data.goalie;

    if !ok {
        HEADING_SIGNAL.signal(0.);
        COORDINATE_SIGNAL.signal((x, y + 5.));
        return;
    }

    let no_ball_distance = if !goalie {
        NO_BALL_DISTANCE
    } else {
        GOALIE_NO_BALL_DISTANCE
    };

    let new_x = FIELD_WIDTH / 2.;
    let new_y = FIELD_LENGTH - FIELD_MARGIN_Y - no_ball_distance;
    let (diff, _) = construct_vector(new_x - x, new_y - y);

    COORDINATE_SIGNAL.signal((new_x, new_y));

    if diff > DISTANCE_THRESHOLD {
        HEADING_SIGNAL.signal(0.);
        return;
    }

    let heading = read_mutex!(HEADING_MUTEX);
    if state.clockwise && heading > MAX_HEADING || !state.clockwise && heading < -MAX_HEADING {
        state.clockwise = !state.clockwise;
    }

    #[allow(clippy::if_same_then_else)]
    let new_heading = if state.clockwise {
        heading //+ 2.
    } else {
        heading //- 2.
    };

    HEADING_SIGNAL.signal(new_heading);
}
