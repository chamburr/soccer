use crate::{
    modules::HEADING_SIGNAL,
    strategy::{Data, COORDINATE_SIGNAL, FIELD_LENGTH, FIELD_MARGIN_Y, FIELD_WIDTH},
};
use num_traits::Float;

const NO_BALL_DISTANCE: f32 = 35.;
const GOALIE_NO_BALL_DISTANCE: f32 = 5.;
const CHECK_DISTANCE: f32 = 5.;
const DISTANCE_THRESHOLD_Y: f32 = 10.;
const DISTANCE_THRESHOLD_X: f32 = 10.;

#[derive(Default)]
pub struct NoBallState {
    pub check_left: bool,
}

pub async fn run(data: Data, state: &mut NoBallState) {
    let (x, y, ok) = data.coordinates;
    let goalie = data.goalie;

    HEADING_SIGNAL.signal(0.);

    if !ok {
        COORDINATE_SIGNAL.signal((x, y + 5.));
        return;
    }

    let no_ball_distance = if !goalie {
        NO_BALL_DISTANCE
    } else {
        GOALIE_NO_BALL_DISTANCE
    };

    let mut new_x = FIELD_WIDTH / 2.;
    let new_y = FIELD_LENGTH - FIELD_MARGIN_Y - no_ball_distance;

    if (new_y - y).abs() > DISTANCE_THRESHOLD_Y || (new_x - x).abs() > DISTANCE_THRESHOLD_X {
        COORDINATE_SIGNAL.signal((new_x, new_y));
        return;
    }

    if x < new_x - CHECK_DISTANCE {
        state.check_left = false;
    } else if x > new_x + CHECK_DISTANCE {
        state.check_left = true;
    }

    if state.check_left {
        new_x = x - 5.;
    } else {
        new_x = x + 5.;
    }

    COORDINATE_SIGNAL.signal((new_x, new_y));
}
